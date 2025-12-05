#!/bin/bash
# QuantumHarmony Validator Monitoring Script
# Run via cron: */5 * * * * /path/to/monitor-validators.sh

set -e

# Configuration
VALIDATORS=(
    "Alice|http://51.79.26.123:9944"
    "Bob|http://51.79.26.168:9944"
    "Charlie|http://209.38.225.4:9944"
)

LOG_FILE="/tmp/qh-monitor.log"
ALERT_FILE="/tmp/qh-alerts.log"
MIN_PEERS=2
MAX_BLOCK_LAG=10

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Timestamp
timestamp() {
    date '+%Y-%m-%d %H:%M:%S'
}

# Log function
log() {
    echo "[$(timestamp)] $1" | tee -a "$LOG_FILE"
}

# Alert function (customize for your alerting system)
alert() {
    local level=$1
    local message=$2
    echo "[$(timestamp)] [$level] $message" | tee -a "$ALERT_FILE"

    # Uncomment to enable Slack webhook alerts:
    # curl -s -X POST -H 'Content-type: application/json' \
    #     --data "{\"text\":\"[$level] QuantumHarmony: $message\"}" \
    #     "$SLACK_WEBHOOK_URL"

    # Uncomment to enable email alerts:
    # echo "$message" | mail -s "[$level] QuantumHarmony Alert" admin@example.com
}

# Check single validator
check_validator() {
    local name=$1
    local url=$2
    local issues=()

    # Check if RPC is responding
    local health_response
    health_response=$(curl -s --max-time 10 -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' \
        "$url" 2>/dev/null) || {
        alert "CRITICAL" "$name ($url) is not responding!"
        return 1
    }

    # Parse health response
    local peers
    peers=$(echo "$health_response" | grep -o '"peers":[0-9]*' | cut -d':' -f2)
    local syncing
    syncing=$(echo "$health_response" | grep -o '"isSyncing":[a-z]*' | cut -d':' -f2)

    # Check peer count
    if [ -n "$peers" ] && [ "$peers" -lt "$MIN_PEERS" ]; then
        issues+=("Low peer count: $peers (min: $MIN_PEERS)")
    fi

    # Check if syncing (should be false for healthy validator)
    if [ "$syncing" = "true" ]; then
        issues+=("Node is syncing")
    fi

    # Get block height
    local block_response
    block_response=$(curl -s --max-time 10 -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
        "$url" 2>/dev/null)

    local block_hex
    block_hex=$(echo "$block_response" | grep -o '"number":"0x[0-9a-f]*"' | cut -d'"' -f4)
    local block_height=0
    if [ -n "$block_hex" ]; then
        block_height=$((block_hex))
    fi

    # Report issues or success
    if [ ${#issues[@]} -gt 0 ]; then
        for issue in "${issues[@]}"; do
            alert "WARNING" "$name: $issue"
        done
        echo -e "${YELLOW}[WARN]${NC} $name: ${issues[*]} (block: $block_height, peers: $peers)"
        return 1
    else
        echo -e "${GREEN}[OK]${NC} $name: block=$block_height, peers=$peers, syncing=$syncing"
        return 0
    fi
}

# Check block heights are in sync
check_block_sync() {
    local heights=()
    local names=()

    for validator in "${VALIDATORS[@]}"; do
        IFS='|' read -r name url <<< "$validator"

        local block_response
        block_response=$(curl -s --max-time 10 -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"chain_getHeader","params":[],"id":1}' \
            "$url" 2>/dev/null)

        local block_hex
        block_hex=$(echo "$block_response" | grep -o '"number":"0x[0-9a-f]*"' | cut -d'"' -f4)
        if [ -n "$block_hex" ]; then
            heights+=($((block_hex)))
            names+=("$name")
        fi
    done

    # Check if heights are within acceptable range
    if [ ${#heights[@]} -ge 2 ]; then
        local max_height=${heights[0]}
        local min_height=${heights[0]}

        for h in "${heights[@]}"; do
            [ "$h" -gt "$max_height" ] && max_height=$h
            [ "$h" -lt "$min_height" ] && min_height=$h
        done

        local lag=$((max_height - min_height))
        if [ "$lag" -gt "$MAX_BLOCK_LAG" ]; then
            alert "WARNING" "Block height lag detected: $lag blocks (max: $max_height, min: $min_height)"
            echo -e "${YELLOW}[WARN]${NC} Block sync lag: $lag blocks"
        else
            echo -e "${GREEN}[OK]${NC} Block sync: all nodes within $lag blocks"
        fi
    fi
}

# Check faucet health
check_faucet() {
    local faucet_url="http://51.79.26.123:8080/health"

    local response
    response=$(curl -s --max-time 10 "$faucet_url" 2>/dev/null) || {
        alert "WARNING" "Faucet is not responding!"
        echo -e "${RED}[FAIL]${NC} Faucet: not responding"
        return 1
    }

    local healthy
    healthy=$(echo "$response" | grep -o '"healthy":true')

    if [ -n "$healthy" ]; then
        echo -e "${GREEN}[OK]${NC} Faucet: healthy"
    else
        alert "WARNING" "Faucet reports unhealthy status"
        echo -e "${YELLOW}[WARN]${NC} Faucet: unhealthy"
    fi
}

# Main monitoring loop
main() {
    echo ""
    echo "=========================================="
    echo "  QuantumHarmony Validator Monitor"
    echo "  $(timestamp)"
    echo "=========================================="
    echo ""

    local failed=0

    # Check each validator
    for validator in "${VALIDATORS[@]}"; do
        IFS='|' read -r name url <<< "$validator"
        check_validator "$name" "$url" || ((failed++))
    done

    echo ""

    # Check block synchronization
    check_block_sync

    echo ""

    # Check faucet
    check_faucet

    echo ""
    echo "=========================================="

    if [ "$failed" -gt 0 ]; then
        echo -e "${YELLOW}Status: $failed validator(s) with issues${NC}"
        exit 1
    else
        echo -e "${GREEN}Status: All systems operational${NC}"
        exit 0
    fi
}

# Run main function
main "$@"
