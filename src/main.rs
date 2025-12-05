//! QuantumHarmony Testnet Faucet
//!
//! A simple HTTP service that distributes testnet tokens for TPS testing.

use anyhow::{anyhow, Result};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

// Faucet configuration
const DRIP_AMOUNT: u128 = 10_000_000_000_000; // 10 tokens (with 12 decimals)
const RATE_LIMIT_SECONDS: i64 = 60; // 1 minute between requests per address
const MAX_PENDING_TXS: usize = 100;

// Validator endpoints
const VALIDATORS: &[&str] = &[
    "http://51.79.26.123:9944",
    "http://51.79.26.168:9944",
    "http://209.38.225.4:9944",
];

// Alice's SPHINCS+ 48-byte SEED (hex encoded)
// Using the seed triggers the gateway's test account lookup path which uses
// get_test_keypair("Alice") with proper caching mechanism
const ALICE_SPHINCS_SEED_HEX: &str = "2eb5fca9ecb08243d333e38adbc99a786edea20f8f88c51b5703754eef4d7a66183e03c1de99dc133c29c5cde6a984f5";

// Alice's SPHINCS+ SS58 address
const ALICE_ADDRESS: &str = "5HDjAbVHMuJzezSccj6eFrEA6nKjonrFRm8h7aTiJXSHP5Qi";

#[derive(Clone)]
struct AppState {
    rate_limits: Arc<DashMap<String, DateTime<Utc>>>,
    pending_txs: Arc<RwLock<Vec<PendingTx>>>,
    active_validator: Arc<RwLock<String>>,
}

#[derive(Clone)]
struct PendingTx {
    to: String,
    amount: u128,
    timestamp: DateTime<Utc>,
}

#[derive(Deserialize)]
struct DripRequest {
    address: String,
}

#[derive(Serialize)]
struct DripResponse {
    success: bool,
    message: String,
    tx_hash: Option<String>,
    amount: String,
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    active_validator: String,
    pending_txs: usize,
    drip_amount: String,
    rate_limit_seconds: i64,
}

#[derive(Serialize)]
struct HealthResponse {
    healthy: bool,
    validators_online: usize,
    block_height: Option<u64>,
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let validator = state.active_validator.read().await.clone();

    // Check validator health
    let client = reqwest::Client::new();
    let mut validators_online = 0;
    let mut block_height = None;

    for validator_url in VALIDATORS {
        let health_req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_health",
            "params": [],
            "id": 1
        });

        if let Ok(resp) = client
            .post(*validator_url)
            .json(&health_req)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            if resp.status().is_success() {
                validators_online += 1;

                // Get block height from first responding validator
                if block_height.is_none() {
                    let block_req = serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "chain_getHeader",
                        "params": [],
                        "id": 1
                    });

                    if let Ok(block_resp) = client
                        .post(*validator_url)
                        .json(&block_req)
                        .timeout(Duration::from_secs(5))
                        .send()
                        .await
                    {
                        if let Ok(json) = block_resp.json::<serde_json::Value>().await {
                            if let Some(number) = json["result"]["number"].as_str() {
                                if let Ok(height) = u64::from_str_radix(&number[2..], 16) {
                                    block_height = Some(height);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let response = HealthResponse {
        healthy: validators_online > 0,
        validators_online,
        block_height,
    };

    if response.healthy {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let validator = state.active_validator.read().await.clone();
    let pending = state.pending_txs.read().await.len();

    Json(StatusResponse {
        status: "running".to_string(),
        active_validator: validator,
        pending_txs: pending,
        drip_amount: format!("{} QHT", DRIP_AMOUNT / 1_000_000_000_000),
        rate_limit_seconds: RATE_LIMIT_SECONDS,
    })
}

async fn drip(
    State(state): State<AppState>,
    Json(request): Json<DripRequest>,
) -> impl IntoResponse {
    let address = request.address.trim().to_string();

    // Validate address format (should start with 5 for Substrate)
    if !address.starts_with('5') || address.len() != 48 {
        return (
            StatusCode::BAD_REQUEST,
            Json(DripResponse {
                success: false,
                message: "Invalid address format. Must be a valid Substrate address starting with '5'".to_string(),
                tx_hash: None,
                amount: "0".to_string(),
            }),
        );
    }

    // Check rate limit
    let now = Utc::now();
    if let Some(last_request) = state.rate_limits.get(&address) {
        let elapsed = now.signed_duration_since(*last_request);
        if elapsed.num_seconds() < RATE_LIMIT_SECONDS {
            let wait_time = RATE_LIMIT_SECONDS - elapsed.num_seconds();
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(DripResponse {
                    success: false,
                    message: format!("Rate limited. Please wait {} seconds", wait_time),
                    tx_hash: None,
                    amount: "0".to_string(),
                }),
            );
        }
    }

    // Check pending tx limit
    {
        let pending = state.pending_txs.read().await;
        if pending.len() >= MAX_PENDING_TXS {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(DripResponse {
                    success: false,
                    message: "Too many pending transactions. Please try again later.".to_string(),
                    tx_hash: None,
                    amount: "0".to_string(),
                }),
            );
        }
    }

    // Submit transaction via RPC
    let validator = state.active_validator.read().await.clone();

    match submit_transfer(&state, &validator, &address, DRIP_AMOUNT).await {
        Ok(tx_hash) => {
            // Update rate limit
            state.rate_limits.insert(address.clone(), now);

            // Add to pending txs
            {
                let mut pending = state.pending_txs.write().await;
                pending.push(PendingTx {
                    to: address.clone(),
                    amount: DRIP_AMOUNT,
                    timestamp: now,
                });
            }

            info!("Drip sent to {}: {} (tx: {})", address, DRIP_AMOUNT, tx_hash);

            (
                StatusCode::OK,
                Json(DripResponse {
                    success: true,
                    message: "Tokens sent successfully!".to_string(),
                    tx_hash: Some(tx_hash),
                    amount: format!("{} QHT", DRIP_AMOUNT / 1_000_000_000_000),
                }),
            )
        }
        Err(e) => {
            warn!("Failed to send drip to {}: {}", address, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DripResponse {
                    success: false,
                    message: format!("Failed to send transaction: {}", e),
                    tx_hash: None,
                    amount: "0".to_string(),
                }),
            )
        }
    }
}

/// Get genesis hash via gateway RPC
async fn get_genesis_hash(validator_url: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "gateway_genesisHash",
        "params": [],
        "id": 1
    });

    let resp = client
        .post(validator_url)
        .json(&req)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;

    if let Some(error) = json.get("error") {
        return Err(anyhow!("RPC error: {}", error["message"].as_str().unwrap_or("Unknown")));
    }

    json["result"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Failed to get genesis hash"))
}

/// Get account nonce via gateway RPC
async fn get_nonce(validator_url: &str, address: &str) -> Result<u32> {
    let client = reqwest::Client::new();

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "gateway_nonce",
        "params": [address],
        "id": 1
    });

    let resp = client
        .post(validator_url)
        .json(&req)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await?;

    if let Some(error) = json.get("error") {
        return Err(anyhow!("RPC error: {}", error["message"].as_str().unwrap_or("Unknown")));
    }

    Ok(json["result"].as_u64().unwrap_or(0) as u32)
}

/// Submit transfer using gateway_submit RPC (handles SPHINCS+ signing internally)
async fn submit_transfer(_state: &AppState, validator_url: &str, to: &str, amount: u128) -> Result<String> {
    let client = reqwest::Client::new();

    // Get genesis hash
    let genesis_hash = get_genesis_hash(validator_url).await?;

    // Get nonce for Alice
    let nonce = get_nonce(validator_url, ALICE_ADDRESS).await?;

    info!(
        "Submitting via gateway_submit: to={}, amount={}, nonce={}, genesis={}",
        to, amount, nonce, &genesis_hash[..16]
    );

    // Use gateway_submit RPC which handles SPHINCS+ signing
    let submit_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "gateway_submit",
        "params": [{
            "from": ALICE_ADDRESS,
            "to": to,
            "amount": amount.to_string(),
            "nonce": nonce,
            "genesisHash": genesis_hash,
            "secretKey": format!("0x{}", ALICE_SPHINCS_SEED_HEX)
        }],
        "id": 1
    });

    let submit_resp = client
        .post(validator_url)
        .json(&submit_req)
        .timeout(Duration::from_secs(60))  // SPHINCS+ signing takes time
        .send()
        .await?;

    let submit_json: serde_json::Value = submit_resp.json().await?;

    if let Some(error) = submit_json.get("error") {
        return Err(anyhow!(
            "RPC error: {}",
            error["message"].as_str().unwrap_or("Unknown error")
        ));
    }

    // gateway_submit returns {"hash": "0x...", "status": "..."}
    let tx_hash = submit_json["result"]["hash"]
        .as_str()
        .or_else(|| submit_json["result"].as_str())
        .ok_or_else(|| anyhow!("No transaction hash returned: {:?}", submit_json))?
        .to_string();

    info!("Transaction submitted: {}", tx_hash);

    Ok(tx_hash)
}

async fn find_active_validator() -> String {
    let client = reqwest::Client::new();

    for validator_url in VALIDATORS {
        let health_req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_health",
            "params": [],
            "id": 1
        });

        if let Ok(resp) = client
            .post(*validator_url)
            .json(&health_req)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            if resp.status().is_success() {
                info!("Found active validator: {}", validator_url);
                return validator_url.to_string();
            }
        }
    }

    // Default to first validator
    VALIDATORS[0].to_string()
}

fn index_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>QuantumHarmony Testnet Faucet</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 50px auto; padding: 20px; background: #0d1117; color: #c9d1d9; }
        h1 { color: #58a6ff; }
        .container { background: #161b22; padding: 30px; border-radius: 12px; border: 1px solid #30363d; }
        input { width: 100%; padding: 12px; margin: 10px 0; border: 1px solid #30363d; border-radius: 6px; font-size: 14px; background: #0d1117; color: #c9d1d9; }
        button { background: #238636; color: white; padding: 12px 24px; border: none; border-radius: 6px; cursor: pointer; font-size: 16px; width: 100%; }
        button:hover { background: #2ea043; }
        button:disabled { background: #21262d; cursor: not-allowed; }
        .result { margin-top: 20px; padding: 15px; border-radius: 6px; }
        .success { background: #238636; }
        .error { background: #da3633; }
        .info { color: #8b949e; font-size: 14px; margin-top: 20px; }
        a { color: #58a6ff; }
    </style>
</head>
<body>
    <div class="container">
        <h1>QuantumHarmony Faucet</h1>
        <p>Get testnet QHT tokens for testing SPHINCS+ transactions</p>

        <input type="text" id="address" placeholder="Enter your Substrate address (starts with 5...)">
        <button onclick="requestTokens()" id="btn">Request 10 QHT</button>

        <div id="result"></div>

        <div class="info">
            <p><strong>Rate limit:</strong> 1 request per minute per address</p>
            <p><strong>Amount:</strong> 10 QHT per request</p>
            <p><a href="https://github.com/Paraxiom/quantumharmony" target="_blank">GitHub</a> | <a href="https://www.youtube.com/@Paraxiom" target="_blank">YouTube</a></p>
        </div>
    </div>

    <script>
        async function requestTokens() {
            const address = document.getElementById('address').value.trim();
            const btn = document.getElementById('btn');
            const result = document.getElementById('result');

            if (!address) {
                result.innerHTML = '<div class="result error">Please enter an address</div>';
                return;
            }

            btn.disabled = true;
            btn.textContent = 'Sending...';
            result.innerHTML = '';

            try {
                const response = await fetch('/drip', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ address })
                });

                const data = await response.json();

                if (data.success) {
                    result.innerHTML = `<div class="result success">
                        <strong>Success!</strong><br>
                        Sent ${data.amount} to your address<br>
                        <small>TX: ${data.tx_hash}</small>
                    </div>`;
                } else {
                    result.innerHTML = `<div class="result error">${data.message}</div>`;
                }
            } catch (e) {
                result.innerHTML = `<div class="result error">Error: ${e.message}</div>`;
            }

            btn.disabled = false;
            btn.textContent = 'Request 10 QHT';
        }

        // Allow Enter key
        document.getElementById('address').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') requestTokens();
        });
    </script>
</body>
</html>"#
}

async fn index() -> impl IntoResponse {
    axum::response::Html(index_html())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting QuantumHarmony Testnet Faucet (SPHINCS+ enabled)...");
    info!("Alice SPHINCS+ address: {}", ALICE_ADDRESS);

    // Find active validator
    let active_validator = find_active_validator().await;
    info!("Using validator: {}", active_validator);

    // Create app state
    let state = AppState {
        rate_limits: Arc::new(DashMap::new()),
        pending_txs: Arc::new(RwLock::new(Vec::new())),
        active_validator: Arc::new(RwLock::new(active_validator)),
    };

    // Build router
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health_check))
        .route("/status", get(status))
        .route("/drip", post(drip))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Faucet listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
