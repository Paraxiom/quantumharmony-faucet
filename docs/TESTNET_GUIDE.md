# QuantumHarmony Testnet User Guide

Welcome to the QuantumHarmony testnet - the world's first post-quantum blockchain using **SPHINCS+** signatures for all transactions.

## Quick Start

### 1. Get Testnet Tokens

Visit the faucet: **http://51.79.26.123:8080**

Or use the API:
```bash
curl -X POST http://51.79.26.123:8080/drip \
  -H "Content-Type: application/json" \
  -d '{"address": "YOUR_SUBSTRATE_ADDRESS"}'
```

You'll receive **10 QHT** per request (rate limited to 1 request per minute).

### 2. Connect to the Network

Use any of these RPC endpoints:

| Node | RPC URL | WebSocket |
|------|---------|-----------|
| Bob (V2) | http://51.79.26.168:9944 | ws://51.79.26.168:9944 |
| Charlie (V3) | http://209.38.225.4:9944 | ws://209.38.225.4:9944 |

### 3. Check Your Balance

```bash
curl -s -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"gateway_balance","params":["YOUR_ADDRESS"],"id":1}' \
  http://51.79.26.168:9944
```

## Network Details

| Property | Value |
|----------|-------|
| **Network Name** | QuantumHarmony Testnet |
| **Native Token** | QHT |
| **Decimals** | 12 |
| **Block Time** | ~6 seconds |
| **Consensus** | Aura (SPHINCS+ signatures) |
| **Finality** | Quantum Coherence Finality Gadget |
| **Genesis Hash** | `0x9ccd45c062353c5e5f07edf7332364b3954110f24d97a4604b3aa250d4122df0` |

## Post-Quantum Cryptography

QuantumHarmony uses **SPHINCS+-SHAKE-128f** for all cryptographic operations:

- **Transactions**: Signed with SPHINCS+ (~49KB signatures)
- **Consensus Votes**: Signed with Falcon-1024 (~1.3KB signatures)
- **Security Level**: NIST Level 1 (128-bit quantum security)

### Why Post-Quantum?

Traditional blockchain signatures (ECDSA, Ed25519) can be broken by quantum computers using Shor's algorithm. SPHINCS+ is a hash-based signature scheme that remains secure even against quantum attacks.

## API Reference

### Get Balance

```bash
curl -s -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"gateway_balance","params":["5YourAddress..."],"id":1}' \
  http://51.79.26.168:9944
```

### Get Nonce

```bash
curl -s -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"gateway_nonce","params":["5YourAddress..."],"id":1}' \
  http://51.79.26.168:9944
```

### Get Genesis Hash

```bash
curl -s -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"gateway_genesisHash","params":[],"id":1}' \
  http://51.79.26.168:9944
```

### Get Block Header

```bash
curl -s -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
  http://51.79.26.168:9944
```

### Check Node Health

```bash
curl -s -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
  http://51.79.26.168:9944
```

## Running Your Own Node

### Prerequisites

- Ubuntu 22.04 or later
- 4GB RAM minimum
- 50GB disk space
- Rust toolchain

### Build from Source

```bash
git clone https://github.com/Paraxiom/quantumharmony.git
cd quantumharmony
cargo build --release
```

### Join the Testnet

```bash
./target/release/quantumharmony-node \
  --chain dev3SpecRaw.json \
  --name "YourNodeName" \
  --base-path /tmp/qh-node \
  --port 30333 \
  --rpc-port 9944 \
  --rpc-cors all \
  --bootnodes '/ip4/51.79.26.168/tcp/30333/p2p/QmNzKqqPY3J78iJfA6HamZ3qRZtTaDoRXL74bGv79CYLqJ'
```

## Performance Characteristics

Due to the computational cost of post-quantum cryptography:

| Metric | Value |
|--------|-------|
| **TPS (Transactions/sec)** | ~5-10 |
| **SPHINCS+ Sign Time** | ~200-300ms |
| **SPHINCS+ Verify Time** | ~50-100ms |
| **Signature Size** | ~49KB |

This is intentional - we prioritize quantum security over raw throughput.

## Troubleshooting

### "Rate limited" error from faucet

Wait 60 seconds between requests to the same address.

### Transaction stuck as pending

SPHINCS+ signature verification takes time. Transactions typically confirm within 2-3 blocks (~12-18 seconds).

### Node not syncing

Ensure you're using the correct chain spec (`dev3SpecRaw.json`) and bootnode addresses.

## Test Accounts

These accounts are pre-funded on the testnet for development:

| Account | Address |
|---------|---------|
| Alice | `5HDjAbVHMuJzezSccj6eFrEA6nKjonrFRm8h7aTiJXSHP5Qi` |
| Bob | `5CAgvufYLRan7pybcGWqTxsxXRAj922Qep6UJmZuVWu8Uv11` |
| Charlie | `5En9M95WwS354QWCM29UyFLsdQgXZ8WzdBvmHa3u6w1bmTS1` |

## Resources

- [GitHub Repository](https://github.com/Paraxiom/quantumharmony)
- [Faucet Source Code](https://github.com/Paraxiom/quantumharmony-faucet)
- [Paraxiom YouTube](https://www.youtube.com/@Paraxiom)

## Support

If you encounter issues:

1. Check the [GitHub Issues](https://github.com/Paraxiom/quantumharmony/issues)
2. Review node logs for error messages
3. Ensure your client supports SPHINCS+ signatures

---

**Note**: This is a testnet. Tokens have no real value. The network may be reset without notice.
