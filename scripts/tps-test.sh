#!/bin/bash
# QuantumHarmony TPS Test Script
# Tests transaction throughput using the faucet with different addresses

set -e

# Configuration
FAUCET_URL="http://51.79.26.123:8080"
NUM_REQUESTS=10
PARALLEL_REQUESTS=5

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "  QuantumHarmony TPS Test"
echo "=========================================="
echo ""

# Get initial block height
INITIAL_BLOCK=$(curl -s -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
    http://51.79.26.168:9944 | grep -o '"number":"0x[0-9a-f]*"' | cut -d'"' -f4)
INITIAL_HEIGHT=$((INITIAL_BLOCK))

echo "Initial block height: $INITIAL_HEIGHT"
echo ""

# Generate test addresses (these won't be used for actual transactions, just for testing rate limits)
# For real TPS testing, we'd need multiple funded accounts
TEST_ADDRESSES=(
    "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL"
    "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy"
    "5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw"
    "5CRgp6q9R4z1z7K8Xqwu3CPpB5GNMxCCkrKqJvbgzVmVV2rR"
    "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"
)

echo "Testing faucet response time with ${#TEST_ADDRESSES[@]} addresses..."
echo ""

# Time a single request
START_TIME=$(date +%s.%N)
RESPONSE=$(curl -s -X POST "$FAUCET_URL/drip" \
    -H "Content-Type: application/json" \
    -d "{\"address\": \"${TEST_ADDRESSES[0]}\"}")
END_TIME=$(date +%s.%N)

DURATION=$(echo "$END_TIME - $START_TIME" | bc)
echo -e "Single request time: ${YELLOW}${DURATION}s${NC}"
echo "Response: $RESPONSE"
echo ""

# Extract success/failure
if echo "$RESPONSE" | grep -q '"success":true'; then
    echo -e "${GREEN}Transaction submitted successfully!${NC}"
    TX_HASH=$(echo "$RESPONSE" | grep -o '"tx_hash":"[^"]*"' | cut -d'"' -f4)
    echo "TX Hash: $TX_HASH"
else
    echo -e "${YELLOW}Request failed or rate limited (expected for subsequent tests)${NC}"
fi

echo ""
echo "=========================================="
echo "  Concurrent Request Test"
echo "=========================================="
echo ""
echo "Sending $PARALLEL_REQUESTS concurrent requests to different addresses..."
echo ""

# Send concurrent requests (these will mostly fail due to rate limits, but tests parallelism)
START_TIME=$(date +%s.%N)
for i in $(seq 1 $PARALLEL_REQUESTS); do
    ADDR="${TEST_ADDRESSES[$((i % ${#TEST_ADDRESSES[@]}))]}"
    curl -s -X POST "$FAUCET_URL/drip" \
        -H "Content-Type: application/json" \
        -d "{\"address\": \"$ADDR\"}" &
done
wait
END_TIME=$(date +%s.%N)

TOTAL_DURATION=$(echo "$END_TIME - $START_TIME" | bc)
REQUESTS_PER_SEC=$(echo "scale=2; $PARALLEL_REQUESTS / $TOTAL_DURATION" | bc)

echo ""
echo "Concurrent test results:"
echo -e "  Total time: ${YELLOW}${TOTAL_DURATION}s${NC}"
echo -e "  Requests/sec: ${YELLOW}${REQUESTS_PER_SEC}${NC}"
echo ""

# Wait for transactions to be included
echo "Waiting 30 seconds for transactions to be included in blocks..."
sleep 30

# Get final block height
FINAL_BLOCK=$(curl -s -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
    http://51.79.26.168:9944 | grep -o '"number":"0x[0-9a-f]*"' | cut -d'"' -f4)
FINAL_HEIGHT=$((FINAL_BLOCK))

BLOCKS_PRODUCED=$((FINAL_HEIGHT - INITIAL_HEIGHT))
echo ""
echo "=========================================="
echo "  Summary"
echo "=========================================="
echo ""
echo "Blocks produced: $BLOCKS_PRODUCED"
echo "Block rate: $(echo "scale=2; $BLOCKS_PRODUCED / 30" | bc) blocks/sec"
echo ""
echo "Note: Faucet is rate-limited to 1 request/60s per address."
echo "For proper TPS testing, use gateway_submit with multiple funded accounts."
echo ""

# Check faucet health
HEALTH=$(curl -s "$FAUCET_URL/health")
echo "Faucet health: $HEALTH"
