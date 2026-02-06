# Soak Testing for Mycelix WebSocket Server

Soak tests (also known as endurance tests) run the server for extended periods to detect:
- Memory leaks
- File descriptor leaks
- Handle leaks
- Performance degradation over time

## Quick Start

```bash
# Run a 1-hour soak test
./soak_test.sh

# Run with load generation
./soak_test.sh --with-load

# Run for 24 hours
./soak_test.sh --duration 24h
```

## Prerequisites

- Bash 4.0+
- curl (for health endpoint checks)
- jq (for JSON parsing)
- k6 (optional, for load generation)
- Python 3.8+ (optional, for analysis tool)

## Files

| File | Description |
|------|-------------|
| `soak_test.sh` | Main soak test runner script |
| `analyze_soak.py` | Python analysis tool for results |

## Test Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DURATION` | `1h` | Test duration (e.g., `1h`, `24h`, `7d`) |
| `SAMPLE_INTERVAL` | `10` | Seconds between metric samples |
| `SERVER_PORT` | `8888` | WebSocket server port |
| `HEALTH_PORT` | `8889` | Health endpoint port |
| `MEMORY_THRESHOLD_MB` | `500` | Memory alert threshold |

### Command Line Options

```bash
./soak_test.sh [options]

Options:
  --duration DURATION    Test duration (e.g., 1h, 24h, 7d)
  --with-load            Run k6 load generator alongside
  --k6-vus NUM           Number of k6 virtual users (default: 50)
  --threshold MB         Memory threshold for alerts (default: 500)
  --output DIR           Output directory for results
  --help                 Show help
```

## Running Tests

### Basic Soak Test (No Load)

Tests the server at idle to detect baseline memory behavior:

```bash
./soak_test.sh --duration 1h
```

### Soak Test with Load

Tests the server under constant load:

```bash
./soak_test.sh --duration 4h --with-load --k6-vus 100
```

### Extended Soak Test

For production validation, run longer tests:

```bash
# 24-hour test
./soak_test.sh --duration 24h --with-load

# 7-day test (run in tmux/screen)
tmux new-session -d -s soak './soak_test.sh --duration 7d --with-load'
```

## Output

Results are saved to `./soak_results/` by default:

```
soak_results/
  metrics_20240115_143022.csv    # Time-series metrics
  server_20240115_143022.log     # Server logs
  summary_20240115_143022.json   # Test summary
  k6_20240115_143022.log         # K6 output (if --with-load)
```

### Metrics CSV Format

```csv
timestamp,elapsed_seconds,rss_kb,rss_mb,vsz_kb,open_fds,threads,cpu_percent,active_connections,messages_received
2024-01-15T14:30:32-05:00,10,45678,44,123456,42,8,2.5,10,150
```

### Summary JSON Format

```json
{
    "test": "soak_test",
    "duration_seconds": 3600,
    "memory": {
        "min_mb": 40,
        "max_mb": 48,
        "avg_mb": 44,
        "growth_percent": 5.2,
        "potential_leak_detected": false
    },
    "file_descriptors": {
        "initial": 35,
        "final": 38,
        "growth": 3
    },
    "results": {
        "passed": true,
        "reason": "All checks passed"
    }
}
```

## Analyzing Results

### Using the Analysis Tool

```bash
# Text report
python analyze_soak.py soak_results/metrics_*.csv

# HTML report with charts
python analyze_soak.py --output report.html soak_results/metrics_*.csv

# JSON output for CI
python analyze_soak.py --json soak_results/metrics_*.csv
```

### Manual Analysis

Using standard tools:

```bash
# Memory trend (first vs last hour)
head -360 metrics.csv | awk -F',' '{sum+=$4} END {print "First hour avg:", sum/NR}'
tail -360 metrics.csv | awk -F',' '{sum+=$4} END {print "Last hour avg:", sum/NR}'

# Peak memory
awk -F',' 'NR>1 {print $4}' metrics.csv | sort -n | tail -1

# FD trend
awk -F',' 'NR>1 {print NR, $6}' metrics.csv | gnuplot -e "plot '-' with lines"
```

## Expected Memory Behavior

### Normal Behavior

The Mycelix WebSocket server should exhibit:

1. **Stable baseline**: Memory should stabilize within 5 minutes of startup
2. **Bounded growth**: Memory may grow slightly with connections but should return to baseline
3. **No linear growth**: Long-term memory should not show linear upward trend

### Typical Memory Profile

| State | Expected RSS |
|-------|-------------|
| Idle (no connections) | 30-50 MB |
| Light load (10 connections) | 40-60 MB |
| Medium load (100 connections) | 50-100 MB |
| Heavy load (1000 connections) | 100-300 MB |

### Warning Signs

- Memory continuously increasing over time (>10% growth per hour)
- Memory not returning to baseline after connections close
- File descriptors continuously increasing
- Thread count unbounded growth

## Troubleshooting

### High Memory Growth Detected

1. Enable debug logging:
   ```bash
   cargo run -p ws-server -- --log-level debug > server.log 2>&1
   ```

2. Use heaptrack for allocation analysis:
   ```bash
   heaptrack ./target/release/mycelix-ws-server
   # Run soak test
   heaptrack_gui heaptrack.*.gz
   ```

3. Check for common issues:
   - Unbounded event history
   - Message queue growth
   - Connection state not cleaned up

### File Descriptor Leaks

1. Monitor in real-time:
   ```bash
   watch -n 1 'ls -l /proc/$(pgrep mycelix-ws-server)/fd | wc -l'
   ```

2. List open files:
   ```bash
   lsof -p $(pgrep mycelix-ws-server)
   ```

### Server Crashes During Test

Check the log file for panic traces:
```bash
grep -A 20 "panic" soak_results/server_*.log
```

## CI/CD Integration

### GitHub Actions Example

```yaml
soak-test:
  runs-on: ubuntu-latest
  timeout-minutes: 120  # 2 hour timeout
  steps:
    - uses: actions/checkout@v4

    - name: Build release
      run: cargo build -p ws-server --release

    - name: Run soak test (1 hour)
      run: |
        cd tests/soak
        ./soak_test.sh --duration 1h

    - name: Upload results
      uses: actions/upload-artifact@v4
      with:
        name: soak-results
        path: tests/soak/soak_results/

    - name: Check results
      run: |
        python tests/soak/analyze_soak.py --json tests/soak/soak_results/metrics_*.csv
```

## Performance Baselines

### Targets

| Metric | Target | Warning | Critical |
|--------|--------|---------|----------|
| Memory growth | <5%/hour | 5-10%/hour | >10%/hour |
| FD growth | <1/hour | 1-5/hour | >5/hour |
| Baseline memory | <100 MB | 100-200 MB | >200 MB |
| Peak memory | <500 MB | 500-1000 MB | >1000 MB |

### Validation Criteria

For a soak test to pass:
1. Memory growth must be < 10% over test duration
2. No file descriptor leaks (growth < 10 FDs)
3. No crashes or panics
4. Server remains responsive throughout
