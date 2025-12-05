# QuantumHarmony Testnet Faucet

A testnet faucet for the QuantumHarmony blockchain, distributing QHT tokens using **SPHINCS+ post-quantum cryptographic signatures**.

## Features

- **Post-Quantum Security**: All transactions are signed with SPHINCS+-SHAKE-128f signatures
- **Rate Limiting**: 1 request per 60 seconds per address
- **Web Interface**: Simple UI for requesting tokens
- **REST API**: Programmatic access for testing tools

## Live Endpoints

| Endpoint | URL |
|----------|-----|
| **Faucet Web UI** | http://51.79.26.123:8080 |
| **API Drip** | POST http://51.79.26.123:8080/drip |
| **Health Check** | GET http://51.79.26.123:8080/health |
| **Status** | GET http://51.79.26.123:8080/status |

## API Usage

### Request Tokens

```bash
curl -X POST http://51.79.26.123:8080/drip \
  -H "Content-Type: application/json" \
  -d '{"address": "5YourSubstrateAddressHere..."}'
```

**Response:**
```json
{
  "success": true,
  "message": "Tokens sent successfully!",
  "tx_hash": "0x8d2d81c396aa5842c76df240371cab572db6700200e1e60afa3306b1d718689e",
  "amount": "10 QHT"
}
```

### Check Health

```bash
curl http://51.79.26.123:8080/health
```

**Response:**
```json
{
  "healthy": true,
  "validators_online": 3,
  "block_height": 1234
}
```

### Check Status

```bash
curl http://51.79.26.123:8080/status
```

**Response:**
```json
{
  "status": "running",
  "active_validator": "http://51.79.26.168:9944",
  "pending_txs": 0,
  "drip_amount": "10 QHT",
  "rate_limit_seconds": 60
}
```

## Configuration

| Parameter | Value | Description |
|-----------|-------|-------------|
| `DRIP_AMOUNT` | 10 QHT | Tokens per request |
| `RATE_LIMIT_SECONDS` | 60 | Cooldown between requests |
| `MAX_PENDING_TXS` | 100 | Maximum pending transactions |

## Testnet Validators

| Node | RPC Endpoint | Status |
|------|--------------|--------|
| Alice (V1) | http://51.79.26.123:9944 | Active |
| Bob (V2) | http://51.79.26.168:9944 | Active |
| Charlie (V3) | http://209.38.225.4:9944 | Active |

## Building from Source

```bash
# Clone the repository
git clone https://github.com/Paraxiom/quantumharmony-faucet.git
cd quantumharmony-faucet

# Build release binary
cargo build --release

# Run the faucet
./target/release/quantumharmony-faucet
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Faucet Service                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Web UI    │  │  REST API   │  │   Rate Limiter      │ │
│  │  (port 8080)│  │  /drip      │  │   (per address)     │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         │                │                     │            │
│         └────────────────┼─────────────────────┘            │
│                          ▼                                  │
│              ┌───────────────────────┐                      │
│              │   gateway_submit RPC  │                      │
│              │   (SPHINCS+ signing)  │                      │
│              └───────────┬───────────┘                      │
└──────────────────────────┼──────────────────────────────────┘
                           ▼
        ┌──────────────────────────────────────┐
        │        QuantumHarmony Testnet        │
        │  ┌────────┐ ┌────────┐ ┌────────┐   │
        │  │ Alice  │ │  Bob   │ │Charlie │   │
        │  │  (V1)  │ │  (V2)  │ │  (V3)  │   │
        │  └────────┘ └────────┘ └────────┘   │
        └──────────────────────────────────────┘
```

## Post-Quantum Cryptography

This faucet uses **SPHINCS+-SHAKE-128f** signatures for all transactions:

- **Signature Size**: ~49 KB (stateless hash-based)
- **Security Level**: NIST Level 1 (128-bit quantum security)
- **Key Format**: 48-byte seed triggers cached keypair lookup

The `gateway_submit` RPC handles SPHINCS+ signing internally, allowing the faucet to submit transactions without managing complex cryptographic operations.

## License

MIT License - See [LICENSE](LICENSE) for details.

## Links

- [QuantumHarmony Main Repo](https://github.com/Paraxiom/quantumharmony)
- [Paraxiom YouTube](https://www.youtube.com/@Paraxiom)
