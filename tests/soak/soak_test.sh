#!/bin/bash
# Soak Test for Mycelix WebSocket RPC Server
# Runs the server for an extended period while monitoring memory and handle usage
#
# Usage:
#   ./soak_test.sh                    # 1 hour test
#   ./soak_test.sh --duration 24h     # 24 hour test
#   ./soak_test.sh --with-load        # With k6 load generator

set -euo pipefail

# Configuration
DURATION="${DURATION:-1h}"
SAMPLE_INTERVAL="${SAMPLE_INTERVAL:-10}"  # seconds
OUTPUT_DIR="${OUTPUT_DIR:-./soak_results}"
SERVER_PORT="${SERVER_PORT:-8888}"
HEALTH_PORT="${HEALTH_PORT:-8889}"
WITH_LOAD="${WITH_LOAD:-false}"
K6_VUS="${K6_VUS:-50}"
MEMORY_THRESHOLD_MB="${MEMORY_THRESHOLD_MB:-500}"  # Alert if memory exceeds this

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --duration)
            DURATION="$2"
            shift 2
            ;;
        --with-load)
            WITH_LOAD="true"
            shift
            ;;
        --k6-vus)
            K6_VUS="$2"
            shift 2
            ;;
        --threshold)
            MEMORY_THRESHOLD_MB="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --duration DURATION    Test duration (e.g., 1h, 24h, 7d)"
            echo "  --with-load            Run k6 load generator alongside"
            echo "  --k6-vus NUM           Number of k6 virtual users (default: 50)"
            echo "  --threshold MB         Memory threshold for alerts (default: 500)"
            echo "  --output DIR           Output directory for results"
            echo "  --help                 Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Convert duration to seconds
duration_to_seconds() {
    local dur=$1
    local num=${dur%[hdms]}
    local unit=${dur: -1}

    case $unit in
        s) echo "$num" ;;
        m) echo $((num * 60)) ;;
        h) echo $((num * 3600)) ;;
        d) echo $((num * 86400)) ;;
        *) echo "$dur" ;;  # Assume seconds if no unit
    esac
}

DURATION_SECONDS=$(duration_to_seconds "$DURATION")

# Create output directory
mkdir -p "$OUTPUT_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
METRICS_FILE="$OUTPUT_DIR/metrics_$TIMESTAMP.csv"
LOG_FILE="$OUTPUT_DIR/server_$TIMESTAMP.log"
SUMMARY_FILE="$OUTPUT_DIR/summary_$TIMESTAMP.json"

echo "=== Mycelix WebSocket Server Soak Test ==="
echo "Duration: $DURATION ($DURATION_SECONDS seconds)"
echo "Sample interval: ${SAMPLE_INTERVAL}s"
echo "Output directory: $OUTPUT_DIR"
echo "Memory threshold: ${MEMORY_THRESHOLD_MB}MB"
echo "With load: $WITH_LOAD"
if [[ "$WITH_LOAD" == "true" ]]; then
    echo "K6 VUs: $K6_VUS"
fi
echo ""

# Check prerequisites
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo "Error: $1 is required but not installed."
        exit 1
    fi
}

check_command cargo

if [[ "$WITH_LOAD" == "true" ]]; then
    check_command k6
fi

# Build the server in release mode
echo "Building server in release mode..."
cargo build -p ws-server --release 2>&1 | tail -5

SERVER_BIN="./target/release/mycelix-ws-server"

if [[ ! -f "$SERVER_BIN" ]]; then
    echo "Error: Server binary not found at $SERVER_BIN"
    exit 1
fi

# Start the server
echo "Starting server..."
$SERVER_BIN --port "$SERVER_PORT" --health-port "$HEALTH_PORT" --log-level info > "$LOG_FILE" 2>&1 &
SERVER_PID=$!

# Wait for server to start
sleep 2

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "Error: Server failed to start. Check $LOG_FILE"
    cat "$LOG_FILE"
    exit 1
fi

echo "Server started with PID: $SERVER_PID"

# Cleanup function
cleanup() {
    echo ""
    echo "Stopping test..."

    if [[ -n "${K6_PID:-}" ]] && kill -0 "$K6_PID" 2>/dev/null; then
        kill "$K6_PID" 2>/dev/null || true
        wait "$K6_PID" 2>/dev/null || true
    fi

    if kill -0 "$SERVER_PID" 2>/dev/null; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi

    echo "Test stopped."
}

trap cleanup EXIT INT TERM

# Start k6 load generator if requested
if [[ "$WITH_LOAD" == "true" ]]; then
    echo "Starting k6 load generator..."
    K6_SCRIPT="$(dirname "$0")/../load/k6/websocket_load.js"

    if [[ -f "$K6_SCRIPT" ]]; then
        k6 run --vus "$K6_VUS" --duration "$DURATION" "$K6_SCRIPT" > "$OUTPUT_DIR/k6_$TIMESTAMP.log" 2>&1 &
        K6_PID=$!
        echo "K6 started with PID: $K6_PID"
    else
        echo "Warning: k6 script not found at $K6_SCRIPT, skipping load generation"
        WITH_LOAD="false"
    fi
fi

# Initialize metrics file
echo "timestamp,elapsed_seconds,rss_kb,rss_mb,vsz_kb,open_fds,threads,cpu_percent,active_connections,messages_received" > "$METRICS_FILE"

# Get process metrics
get_metrics() {
    local pid=$1

    # Memory (RSS and VSZ in KB)
    local mem_info
    mem_info=$(ps -o rss=,vsz= -p "$pid" 2>/dev/null || echo "0 0")
    local rss_kb vsz_kb
    read -r rss_kb vsz_kb <<< "$mem_info"

    # File descriptors
    local open_fds=0
    if [[ -d "/proc/$pid/fd" ]]; then
        open_fds=$(ls -1 "/proc/$pid/fd" 2>/dev/null | wc -l)
    elif command -v lsof &> /dev/null; then
        open_fds=$(lsof -p "$pid" 2>/dev/null | wc -l)
    fi

    # Threads
    local threads=0
    if [[ -d "/proc/$pid/task" ]]; then
        threads=$(ls -1 "/proc/$pid/task" 2>/dev/null | wc -l)
    fi

    # CPU (approximation)
    local cpu_percent
    cpu_percent=$(ps -o %cpu= -p "$pid" 2>/dev/null | tr -d ' ' || echo "0")

    # Server metrics from health endpoint
    local active_connections=0
    local messages_received=0
    if curl -s "http://localhost:$HEALTH_PORT/metrics" > /tmp/metrics.json 2>/dev/null; then
        active_connections=$(jq -r '.activeConnections // 0' /tmp/metrics.json 2>/dev/null || echo "0")
        messages_received=$(jq -r '.messagesReceived // 0' /tmp/metrics.json 2>/dev/null || echo "0")
    fi

    echo "$rss_kb,$vsz_kb,$open_fds,$threads,$cpu_percent,$active_connections,$messages_received"
}

# Main monitoring loop
echo ""
echo "Starting monitoring loop..."
echo "Press Ctrl+C to stop early"
echo ""

START_TIME=$(date +%s)
SAMPLE_COUNT=0
MAX_RSS_KB=0
MIN_RSS_KB=999999999
ALERT_COUNT=0

while true; do
    CURRENT_TIME=$(date +%s)
    ELAPSED=$((CURRENT_TIME - START_TIME))

    if [[ $ELAPSED -ge $DURATION_SECONDS ]]; then
        echo "Duration reached, stopping test..."
        break
    fi

    # Check if server is still running
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        echo "ERROR: Server process died unexpectedly!"
        echo "Check $LOG_FILE for details"
        break
    fi

    # Get metrics
    TIMESTAMP=$(date -Iseconds)
    METRICS=$(get_metrics "$SERVER_PID")
    IFS=',' read -r RSS_KB VSZ_KB OPEN_FDS THREADS CPU ACTIVE_CONNS MSGS_RECV <<< "$METRICS"
    RSS_MB=$((RSS_KB / 1024))

    # Update min/max
    if [[ $RSS_KB -gt $MAX_RSS_KB ]]; then
        MAX_RSS_KB=$RSS_KB
    fi
    if [[ $RSS_KB -lt $MIN_RSS_KB ]]; then
        MIN_RSS_KB=$RSS_KB
    fi

    # Write to CSV
    echo "$TIMESTAMP,$ELAPSED,$RSS_KB,$RSS_MB,$VSZ_KB,$OPEN_FDS,$THREADS,$CPU,$ACTIVE_CONNS,$MSGS_RECV" >> "$METRICS_FILE"

    # Check memory threshold
    if [[ $RSS_MB -gt $MEMORY_THRESHOLD_MB ]]; then
        echo "[ALERT] Memory threshold exceeded: ${RSS_MB}MB > ${MEMORY_THRESHOLD_MB}MB"
        ALERT_COUNT=$((ALERT_COUNT + 1))
    fi

    # Progress update every minute
    SAMPLE_COUNT=$((SAMPLE_COUNT + 1))
    if [[ $((SAMPLE_COUNT % 6)) -eq 0 ]]; then
        PERCENT=$((ELAPSED * 100 / DURATION_SECONDS))
        echo "[$(date '+%H:%M:%S')] Progress: ${PERCENT}% | RSS: ${RSS_MB}MB | FDs: $OPEN_FDS | Threads: $THREADS | Conns: $ACTIVE_CONNS"
    fi

    sleep "$SAMPLE_INTERVAL"
done

# Calculate statistics
echo ""
echo "Calculating statistics..."

MAX_RSS_MB=$((MAX_RSS_KB / 1024))
MIN_RSS_MB=$((MIN_RSS_KB / 1024))

# Calculate average and detect leaks
AVG_RSS_KB=$(awk -F',' 'NR>1 {sum+=$3; count++} END {print int(sum/count)}' "$METRICS_FILE")
AVG_RSS_MB=$((AVG_RSS_KB / 1024))

# Get first and last 10% of samples for leak detection
TOTAL_SAMPLES=$((SAMPLE_COUNT))
FIRST_SAMPLES=$((TOTAL_SAMPLES / 10))
LAST_SAMPLES=$((TOTAL_SAMPLES / 10))

FIRST_AVG=$(awk -F',' -v n="$FIRST_SAMPLES" 'NR>1 && NR<=n+1 {sum+=$3; count++} END {print int(sum/count)}' "$METRICS_FILE")
LAST_AVG=$(awk -F',' -v n="$LAST_SAMPLES" -v t="$TOTAL_SAMPLES" 'NR>t-n+1 {sum+=$3; count++} END {print int(sum/count)}' "$METRICS_FILE")

MEMORY_GROWTH=$((LAST_AVG - FIRST_AVG))
MEMORY_GROWTH_PERCENT=0
if [[ $FIRST_AVG -gt 0 ]]; then
    MEMORY_GROWTH_PERCENT=$((MEMORY_GROWTH * 100 / FIRST_AVG))
fi

# Determine if there's a potential leak (>10% growth)
LEAK_DETECTED="false"
if [[ $MEMORY_GROWTH_PERCENT -gt 10 ]]; then
    LEAK_DETECTED="true"
fi

# Get final FD count
FINAL_FDS=$(tail -1 "$METRICS_FILE" | cut -d',' -f6)
INITIAL_FDS=$(head -2 "$METRICS_FILE" | tail -1 | cut -d',' -f6)
FD_GROWTH=$((FINAL_FDS - INITIAL_FDS))

# Write summary
cat > "$SUMMARY_FILE" << EOF
{
    "test": "soak_test",
    "timestamp": "$(date -Iseconds)",
    "duration_seconds": $DURATION_SECONDS,
    "duration_requested": "$DURATION",
    "sample_count": $SAMPLE_COUNT,
    "sample_interval_seconds": $SAMPLE_INTERVAL,
    "with_load": $WITH_LOAD,
    "k6_vus": $K6_VUS,
    "memory": {
        "min_mb": $MIN_RSS_MB,
        "max_mb": $MAX_RSS_MB,
        "avg_mb": $AVG_RSS_MB,
        "first_10pct_avg_kb": $FIRST_AVG,
        "last_10pct_avg_kb": $LAST_AVG,
        "growth_kb": $MEMORY_GROWTH,
        "growth_percent": $MEMORY_GROWTH_PERCENT,
        "threshold_mb": $MEMORY_THRESHOLD_MB,
        "threshold_exceeded_count": $ALERT_COUNT,
        "potential_leak_detected": $LEAK_DETECTED
    },
    "file_descriptors": {
        "initial": $INITIAL_FDS,
        "final": $FINAL_FDS,
        "growth": $FD_GROWTH
    },
    "results": {
        "passed": $([ "$LEAK_DETECTED" == "false" ] && [ "$ALERT_COUNT" -eq 0 ] && echo "true" || echo "false"),
        "reason": "$([ "$LEAK_DETECTED" == "true" ] && echo "Potential memory leak detected" || ([ "$ALERT_COUNT" -gt 0 ] && echo "Memory threshold exceeded $ALERT_COUNT times" || echo "All checks passed"))"
    },
    "files": {
        "metrics": "$METRICS_FILE",
        "log": "$LOG_FILE",
        "summary": "$SUMMARY_FILE"
    }
}
EOF

# Print summary
echo ""
echo "=== Soak Test Summary ==="
echo ""
echo "Duration: $DURATION ($SAMPLE_COUNT samples)"
echo ""
echo "Memory (RSS):"
echo "  Min:     ${MIN_RSS_MB}MB"
echo "  Max:     ${MAX_RSS_MB}MB"
echo "  Average: ${AVG_RSS_MB}MB"
echo "  Growth:  ${MEMORY_GROWTH}KB (${MEMORY_GROWTH_PERCENT}%)"
echo ""
echo "File Descriptors:"
echo "  Initial: $INITIAL_FDS"
echo "  Final:   $FINAL_FDS"
echo "  Growth:  $FD_GROWTH"
echo ""

if [[ "$LEAK_DETECTED" == "true" ]]; then
    echo "WARNING: Potential memory leak detected!"
    echo "Memory grew by ${MEMORY_GROWTH_PERCENT}% over the test duration."
    echo ""
fi

if [[ $ALERT_COUNT -gt 0 ]]; then
    echo "WARNING: Memory threshold (${MEMORY_THRESHOLD_MB}MB) exceeded $ALERT_COUNT times"
    echo ""
fi

if [[ "$LEAK_DETECTED" == "false" ]] && [[ $ALERT_COUNT -eq 0 ]]; then
    echo "PASSED: No memory leaks or threshold violations detected"
else
    echo "FAILED: Issues detected during soak test"
    exit 1
fi

echo ""
echo "Results saved to:"
echo "  Metrics: $METRICS_FILE"
echo "  Log:     $LOG_FILE"
echo "  Summary: $SUMMARY_FILE"
