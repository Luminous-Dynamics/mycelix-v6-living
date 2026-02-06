# k6 Load Tests for Mycelix WebSocket RPC Server

This directory contains k6 load tests for the Mycelix Living Protocol WebSocket RPC server.

## Prerequisites

### Installing k6

**macOS (Homebrew):**
```bash
brew install k6
```

**Linux (Debian/Ubuntu):**
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Docker:**
```bash
docker pull grafana/k6
```

**Windows:**
```bash
winget install k6
# or
choco install k6
```

## Test Files

| File | Description |
|------|-------------|
| `config.js` | Shared configuration and utilities |
| `websocket_load.js` | WebSocket connection load test (concurrent connections) |
| `rpc_throughput.js` | RPC throughput test (max requests/second) |
| `connection_churn.js` | Connection open/close stress test |

## Running Tests

### Start the Server

First, start the Mycelix WebSocket server:

```bash
# From project root
cargo run -p ws-server --release

# Or with simulated time for faster testing
cargo run -p ws-server --release -- --simulated-time --time-acceleration 100
```

### Basic Usage

```bash
# Run WebSocket load test
k6 run websocket_load.js

# Run RPC throughput test
k6 run rpc_throughput.js

# Run connection churn test
k6 run connection_churn.js
```

### Custom Configuration

Override defaults with environment variables:

```bash
# Custom server address
WS_HOST=192.168.1.100 WS_PORT=9000 k6 run websocket_load.js

# Custom load parameters
MAX_VUS=500 k6 run websocket_load.js

# Custom thresholds
P50_THRESHOLD=5 P95_THRESHOLD=20 P99_THRESHOLD=50 k6 run websocket_load.js

# Throughput test parameters
ITERATIONS=50000 THROUGHPUT_DURATION=5m k6 run rpc_throughput.js

# Churn test parameters
CONN_RATE=100 CHURN_DURATION=10m k6 run connection_churn.js
```

### Docker

```bash
# Run with Docker
docker run --rm -i --network host \
  -v $(pwd):/scripts \
  grafana/k6 run /scripts/websocket_load.js
```

## Environment Variables

### Server Configuration
| Variable | Default | Description |
|----------|---------|-------------|
| `WS_HOST` | `localhost` | WebSocket server host |
| `WS_PORT` | `8888` | WebSocket server port |
| `HEALTH_PORT` | `8889` | Health endpoint port |

### Load Parameters
| Variable | Default | Description |
|----------|---------|-------------|
| `MAX_VUS` | `1000` | Maximum virtual users (websocket test) |
| `ITERATIONS` | `10000` | Total requests (throughput test) |
| `THROUGHPUT_DURATION` | `2m` | Throughput test duration |
| `CONN_RATE` | `50` | Connections per second (churn test) |
| `CHURN_DURATION` | `5m` | Churn test duration |

### Thresholds
| Variable | Default | Description |
|----------|---------|-------------|
| `P50_THRESHOLD` | `10` | p50 latency threshold (ms) |
| `P95_THRESHOLD` | `50` | p95 latency threshold (ms) |
| `P99_THRESHOLD` | `100` | p99 latency threshold (ms) |
| `ERROR_RATE_THRESHOLD` | `0.1` | Max error rate (%) |
| `MIN_RPS` | `1000` | Minimum requests per second |

## Output

Results are written to `results/` directory:
- `websocket_load.json` - WebSocket load test results
- `rpc_throughput.json` - Throughput test results
- `connection_churn.json` - Connection churn test results

### Example Output

```
=== WebSocket Load Test Summary ===

RPC Request Duration:
  p(50): 2.34ms
  p(95): 8.67ms
  p(99): 15.23ms
  avg:   3.45ms
  max:   45.12ms

Total RPC Requests: 125432
Error Rate: 0.01%
Connections Opened: 1000
Connections Failed: 2
```

## Interpreting Results

### Key Metrics

1. **RPC Request Duration** (latency)
   - `p(50)`: Median latency - 50% of requests complete within this time
   - `p(95)`: 95th percentile - only 5% of requests take longer
   - `p(99)`: 99th percentile - outliers, should still be reasonable

2. **Error Rate**
   - Should be < 0.1% under normal load
   - Increase may indicate server overload or bugs

3. **Throughput (RPS)**
   - Requests per second the server can handle
   - Should be > 1000 RPS for single connection

4. **Connection Metrics**
   - Handshake time: Time to establish WebSocket connection
   - Connection duration: Total time connection was open

### Performance Baselines

| Metric | Target | Warning | Critical |
|--------|--------|---------|----------|
| p50 latency | < 10ms | 10-20ms | > 20ms |
| p95 latency | < 50ms | 50-100ms | > 100ms |
| p99 latency | < 100ms | 100-200ms | > 200ms |
| Error rate | < 0.1% | 0.1-1% | > 1% |
| Throughput | > 1000 RPS | 500-1000 RPS | < 500 RPS |
| Handshake | < 100ms | 100-500ms | > 500ms |

### Troubleshooting

**High latency:**
- Check server CPU utilization
- Look for lock contention in cycle engine
- Consider scaling horizontally

**Connection failures:**
- Check file descriptor limits (`ulimit -n`)
- Verify server is not rate limiting
- Check for network issues

**Throughput plateau:**
- Single-threaded bottleneck in message handling
- Consider async batching
- Profile with flamegraph

## CI/CD Integration

The tests output JSON results that can be parsed in CI:

```bash
# Run test and check exit code
k6 run --out json=results.json websocket_load.js

# Check thresholds passed
if [ $? -eq 0 ]; then
  echo "All thresholds passed"
else
  echo "Performance regression detected"
  exit 1
fi
```

## Advanced Usage

### Grafana Integration

Stream metrics to Grafana Cloud or InfluxDB:

```bash
# InfluxDB
k6 run --out influxdb=http://localhost:8086/k6 websocket_load.js

# Grafana Cloud
K6_CLOUD_TOKEN=your-token k6 cloud websocket_load.js
```

### Custom Scenarios

Modify `options.scenarios` in test files for custom load patterns:

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
