# Load Testing Suite for Mycelix Living Protocol

Comprehensive load, performance, and endurance testing for the Mycelix WebSocket RPC server.

## Overview

This test suite includes:

1. **k6 Load Tests** (`k6/`) - WebSocket connection and RPC load testing
2. **Rust Benchmarks** (`benches/ws_server.rs`) - Micro-benchmarks for RPC operations
3. **Soak Tests** (`../soak/`) - Extended duration memory leak detection

## Quick Start

### Prerequisites

```bash
# Install k6 (macOS)
brew install k6

# Install k6 (Linux)
sudo apt-get install k6

# Ensure Rust benchmarks work
cargo bench --bench ws_server --no-run
```

### Running Tests

```bash
# Start the server
cargo run -p ws-server --release

# In another terminal:

# Quick load test (5 minutes)
k6 run tests/load/k6/websocket_load.js

# Throughput test
k6 run tests/load/k6/rpc_throughput.js

# Connection churn test
k6 run tests/load/k6/connection_churn.js

# Rust benchmarks
cargo bench --bench ws_server
```

## Test Descriptions

### 1. WebSocket Load Test (`websocket_load.js`)

Tests concurrent WebSocket connections with RPC requests.

**Stages:**
1. Warm up: Ramp to 100 VUs over 30s
2. Ramp up: Scale to 500 VUs over 1m
3. Full load: Reach 1000 VUs over 2m
4. Steady state: Hold 1000 VUs for 5m
5. Ramp down: Scale to 0 over 1.5m

**Metrics Measured:**
- `rpc_request_duration`: Latency percentiles (p50, p95, p99)
- `rpc_errors`: Error rate
- `connections_opened/failed`: Connection success rate

**Usage:**
```bash
# Default (1000 concurrent connections)
k6 run websocket_load.js

# Custom configuration
MAX_VUS=500 P99_THRESHOLD=50 k6 run websocket_load.js
```

### 2. RPC Throughput Test (`rpc_throughput.js`)

Measures maximum requests per second on a single connection.

**What it tests:**
- Maximum sustainable RPS
- Latency under high throughput
- Message ordering and reliability

**Usage:**
```bash
# Default (10,000 iterations over 2 minutes)
k6 run rpc_throughput.js

# Extended test
ITERATIONS=100000 THROUGHPUT_DURATION=10m k6 run rpc_throughput.js
```

### 3. Connection Churn Test (`connection_churn.js`)

Stress tests connection lifecycle handling.

**Lifecycle patterns tested:**
- Immediate close (10%)
- Single request (30%)
- Burst requests (20%)
- Normal session (25%)
- Long session (15%)

**Usage:**
```bash
# Default (50 connections/second for 5 minutes)
k6 run connection_churn.js

# Higher rate
CONN_RATE=200 CHURN_DURATION=10m k6 run connection_churn.js
```

### 4. Rust Benchmarks (`benches/ws_server.rs`)

Micro-benchmarks using Criterion for statistical analysis.

**Benchmarks included:**
- `rpc_request_parsing`: JSON deserialization of RPC requests
- `rpc_response_serialization`: JSON serialization of responses
- `request_handling`: Full request-response cycle
- `concurrent_handling`: Multi-threaded request processing
- `param_extraction`: JSON parameter access
- `string_operations`: ID generation, method matching

**Usage:**
```bash
# Run all WebSocket benchmarks
cargo bench --bench ws_server

# Run specific benchmark
cargo bench --bench ws_server -- request_parsing

# Generate HTML report
cargo bench --bench ws_server -- --save-baseline main
```

## Performance Baselines

### Latency Targets

| Percentile | Target | Warning | Critical |
|------------|--------|---------|----------|
| p50 | < 10ms | 10-20ms | > 20ms |
| p95 | < 50ms | 50-100ms | > 100ms |
| p99 | < 100ms | 100-200ms | > 200ms |

### Throughput Targets

| Metric | Target | Warning | Critical |
|--------|--------|---------|----------|
| Single connection RPS | > 1,000 | 500-1,000 | < 500 |
| Concurrent connections | > 1,000 | 500-1,000 | < 500 |
| Connection rate | > 100/s | 50-100/s | < 50/s |

### Error Rates

| Metric | Target | Warning | Critical |
|--------|--------|---------|----------|
| RPC error rate | < 0.1% | 0.1-1% | > 1% |
| Connection failure | < 0.1% | 0.1-1% | > 1% |

## CI/CD Integration

### Running in CI

```yaml
# GitHub Actions example
performance-tests:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Install k6
      run: |
        sudo gpg -k
        sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg \
          --keyserver hkp://keyserver.ubuntu.com:80 \
          --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
        echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" \
          | sudo tee /etc/apt/sources.list.d/k6.list
        sudo apt-get update && sudo apt-get install k6

    - name: Build server
      run: cargo build -p ws-server --release

    - name: Start server
      run: |
        ./target/release/mycelix-ws-server &
        sleep 5

    - name: Run load tests
      run: |
        k6 run --out json=results.json tests/load/k6/websocket_load.js

    - name: Check thresholds
      run: |
        # k6 exits with non-zero if thresholds fail
        if [ $? -ne 0 ]; then
          echo "Performance regression detected!"
          exit 1
        fi
```

### Comparing Against Baseline

```bash
# Save baseline
cargo bench --bench ws_server -- --save-baseline main

# Compare against baseline
cargo bench --bench ws_server -- --baseline main

# Fail if regression > 10%
cargo bench --bench ws_server -- --baseline main --noplot 2>&1 | \
  grep -E "regressed|improved" | \
  awk '{if ($NF > 10) exit 1}'
```

## Interpreting Results

### k6 Output

```
     scenarios: (100.00%) 1 scenario, 1000 max VUs, 10m30s max duration
                websocket_load: 1000 looping VUs for 10m0s

     ✓ WebSocket connected successfully

     checks.........................: 99.98% ✓ 125430   ✗ 25
     rpc_errors.....................: 0.01%  ✓ 12       ✗ 125418
     rpc_request_duration...........: avg=3.45ms min=0.5ms med=2.1ms max=45ms p(95)=8.67ms p(99)=15.23ms
```

**Key metrics:**
- `checks`: Should be > 99%
- `rpc_errors`: Should be < 0.1%
- `rpc_request_duration p(99)`: Should be < 100ms

### Criterion Output

```
request_handling/handle/getCycleState
                        time:   [1.2345 µs 1.2456 µs 1.2567 µs]
                        change: [-2.1234% +0.1234% +2.3456%] (p = 0.12 > 0.05)
                        No change in performance detected.
```

**Interpretation:**
- `time`: [lower bound, estimate, upper bound]
- `change`: Performance change from baseline
- `p value`: Statistical significance

## Troubleshooting

### High Latency

1. Check server CPU usage
2. Look for lock contention with `perf`
3. Profile with flamegraph

```bash
# Generate flamegraph
cargo install flamegraph
cargo flamegraph --bin mycelix-ws-server
```

### Connection Failures

1. Check ulimits: `ulimit -n`
2. Increase file descriptor limit
3. Check for port exhaustion

```bash
# Increase ulimit
ulimit -n 65535

# Check open connections
ss -s
```

### k6 Errors

1. Check k6 has enough resources
2. Reduce VU count
3. Increase ramp-up time

```bash
# Run with debug output
k6 run --http-debug websocket_load.js
```

## Advanced Usage

### Custom Test Scenarios

Create custom scenarios by modifying options:

```javascript
export const options = {
  scenarios: {
    spike_test: {
      executor: 'ramping-vus',
      stages: [
        { duration: '10s', target: 1000 },  // Instant spike
        { duration: '1m', target: 1000 },   // Hold
        { duration: '10s', target: 0 },     // Drop
      ],
    },
  },
};
```

### Grafana Integration

Stream k6 metrics to Grafana:

```bash
# InfluxDB output
k6 run --out influxdb=http://localhost:8086/k6 websocket_load.js

# Grafana Cloud
K6_CLOUD_TOKEN=your-token k6 cloud websocket_load.js
```

### Distributed Testing

For higher loads, run k6 in distributed mode:

```bash
# On multiple machines
k6 run --execution-segment "0:1/3" websocket_load.js  # Machine 1
k6 run --execution-segment "1/3:2/3" websocket_load.js  # Machine 2
k6 run --execution-segment "2/3:1" websocket_load.js  # Machine 3
```

## Related Documentation

- [k6 Documentation](https://k6.io/docs/)
- [Criterion User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Soak Testing Guide](../soak/README.md)
