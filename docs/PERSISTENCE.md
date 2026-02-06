# Mycelix Persistence Layer

The Mycelix WebSocket server includes a persistence layer for storing cycle history, phase transitions, metrics snapshots, and events. This enables historical analysis, debugging, and recovery after restarts.

## Quick Start

### SQLite (Default)

SQLite is the default database and requires no additional setup:

```bash
# Start server with default SQLite database (./mycelix.db)
cargo run -p ws-server --features sqlite

# Custom database path
cargo run -p ws-server --features sqlite -- --database-url sqlite:./data/mycelix.db
```

### PostgreSQL

For production deployments with high volume, PostgreSQL provides better performance and advanced features:

```bash
# Start server with PostgreSQL
cargo run -p ws-server --features postgres -- \
    --database-url "postgres://user:password@localhost:5432/mycelix"
```

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `--database-url` | `sqlite:./mycelix.db` | Database connection URL |
| `--metrics-retention-days` | `30` | How long to keep historical data |
| `--metrics-snapshot-interval` | `60` | Seconds between metrics snapshots |
| `--no-auto-migrate` | false | Disable automatic schema migrations |

## Database Schema

### Tables

#### `cycle_history`
Tracks overall cycle information:
- `cycle_number`: Unique cycle identifier
- `started_at`: When the cycle began
- `ended_at`: When the cycle ended (NULL if ongoing)
- `total_transitions`: Number of phase transitions in this cycle

#### `phase_transitions`
Records each phase transition with a metrics snapshot:
- `from_phase`: Previous phase name
- `to_phase`: New phase name
- `cycle_number`: Parent cycle
- `transitioned_at`: Timestamp of transition
- `metrics_json`: JSON blob of PhaseMetrics at transition time

#### `metrics_snapshots`
Periodic snapshots of server and cycle metrics:
- Server metrics: `active_connections`, `total_connections`, `messages_received`, `messages_sent`
- Cycle metrics: `spectral_k`, `mean_metabolic_trust`, `active_wounds`, etc.
- `snapshot_at`: When the snapshot was taken

#### `events`
All Living Protocol events for replay/analysis:
- `event_type`: Type of event (e.g., "PhaseTransitioned", "WoundCreated")
- `event_data`: JSON serialized event data
- `cycle_number`: Parent cycle
- `phase`: Phase when event occurred

### Views

- `v_current_cycle`: Current cycle state and phase
- `v_daily_metrics`: Daily aggregated metrics
- `v_phase_durations`: How long each phase lasted

## Feature Flags

The persistence layer uses Cargo features to control which database backends are compiled:

```toml
[features]
default = ["sqlite"]       # SQLite enabled by default
sqlite = ["sqlx/sqlite"]   # SQLite support
postgres = ["sqlx/postgres"] # PostgreSQL support
```

To compile with both backends:
```bash
cargo build -p ws-server --features "sqlite,postgres"
```

## PostgreSQL Setup

### 1. Create Database

```sql
CREATE DATABASE mycelix;
CREATE USER mycelix WITH PASSWORD 'your-password';
GRANT ALL PRIVILEGES ON DATABASE mycelix TO mycelix;
```

### 2. Run Migrations

Migrations run automatically on startup unless `--no-auto-migrate` is specified.

To run manually:
```bash
psql -d mycelix -f migrations/001_initial_pg.sql
```

### 3. Connection Pooling

The server uses connection pooling with configurable limits:
- Default max connections: 10
- Default min idle connections: 1
- Connection timeout: 30 seconds
- Idle timeout: 600 seconds (10 minutes)

For high-load deployments, consider using PgBouncer in front of PostgreSQL.

## Data Retention

Historical data is automatically cleaned up based on the retention period:

```bash
# Keep 90 days of history
cargo run -p ws-server -- --metrics-retention-days 90
```

The cleanup job runs periodically and removes:
- Phase transitions older than retention period
- Metrics snapshots older than retention period
- Events older than retention period

## Querying Data

### SQLite

```bash
# Connect to SQLite database
sqlite3 mycelix.db

# View current cycle
SELECT * FROM v_current_cycle;

# View phase durations
SELECT * FROM v_phase_durations LIMIT 20;

# View daily metrics
SELECT * FROM v_daily_metrics;
```

### PostgreSQL

```bash
# Connect to PostgreSQL
psql -d mycelix

# View current cycle
SELECT * FROM v_current_cycle;

# Get phase statistics for last 7 days
SELECT * FROM get_phase_stats(NOW() - INTERVAL '7 days', NOW());

# Get metrics trend (hourly buckets)
SELECT * FROM get_metrics_trend('spectral_k', NOW() - INTERVAL '24 hours', NOW(), 1);
```

## Backup and Restore

### SQLite

```bash
# Backup
sqlite3 mycelix.db ".backup 'mycelix_backup.db'"

# Restore
cp mycelix_backup.db mycelix.db
```

### PostgreSQL

```bash
# Backup
pg_dump mycelix > mycelix_backup.sql

# Restore
psql -d mycelix < mycelix_backup.sql
```

## Performance Tuning

### SQLite

For better write performance with SQLite:

```sql
-- Enable WAL mode (done automatically)
PRAGMA journal_mode = WAL;

-- Increase cache size (in pages, default page = 4KB)
PRAGMA cache_size = -64000;  -- 64MB cache

-- Synchronous mode (tradeoff between safety and speed)
PRAGMA synchronous = NORMAL;
```

### PostgreSQL

For high-volume deployments:

1. **Partitioning**: Use the provided partition functions for time-series data
2. **BRIN indexes**: Already created for time-series queries
3. **Connection pooling**: Use PgBouncer for many concurrent connections
4. **Autovacuum tuning**: Adjust for write-heavy workloads

```sql
-- Create monthly partition for metrics
SELECT create_metrics_partition('2024-01-01'::DATE);

-- Run manual cleanup
SELECT * FROM cleanup_old_data(30);
```

## Monitoring

### Health Check

The persistence layer reports database health via the `/health` endpoint:

```bash
curl http://localhost:8889/health
# {"status":"healthy","database":"connected"}
```

### Metrics

Database-related metrics are exposed in Prometheus format at `/metrics`:

```
# HELP mycelix_db_connections_active Active database connections
mycelix_db_connections_active 5

# HELP mycelix_db_queries_total Total database queries executed
mycelix_db_queries_total 1234
```

## Troubleshooting

### "database is locked" (SQLite)

This can happen with concurrent writes. Solutions:
1. Enable WAL mode (done automatically)
2. Increase busy timeout
3. Use PostgreSQL for high-concurrency

### Connection pool exhausted

Increase pool size or reduce connection hold time:
```bash
cargo run -p ws-server -- --db-max-connections 20
```

### Slow queries

Check for missing indexes:
```sql
-- SQLite
EXPLAIN QUERY PLAN SELECT * FROM phase_transitions WHERE cycle_number = 1;

-- PostgreSQL
EXPLAIN ANALYZE SELECT * FROM phase_transitions WHERE cycle_number = 1;
```

## API Reference

### Repository Trait

```rust
#[async_trait]
pub trait CycleRepository: Send + Sync {
    async fn save_transition(&self, transition: &PhaseTransition) -> Result<i64>;
    async fn get_history(&self, from: DateTime<Utc>, to: DateTime<Utc>, limit: Option<u32>) -> Result<Vec<StoredPhaseTransition>>;
    async fn get_recent_transitions(&self, limit: u32) -> Result<Vec<StoredPhaseTransition>>;
    async fn save_metrics(&self, cycle_number: u64, phase: CyclePhase, server_metrics: &ServerMetrics, phase_metrics: &PhaseMetrics) -> Result<i64>;
    async fn get_metrics_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<StoredMetricsSnapshot>>;
    async fn get_latest_metrics(&self) -> Result<Option<StoredMetricsSnapshot>>;
    async fn save_event(&self, event_type: &str, event_data: &str, cycle_number: u64, phase: CyclePhase) -> Result<i64>;
    async fn get_events(&self, from: DateTime<Utc>, to: DateTime<Utc>, event_type: Option<&str>) -> Result<Vec<StoredEvent>>;
    async fn cleanup_old_data(&self, retention_days: u32) -> Result<u64>;
    async fn get_last_cycle_number(&self) -> Result<Option<u64>>;
    async fn get_last_phase(&self) -> Result<Option<CyclePhase>>;
}
```

### Configuration

```rust
pub struct PersistenceConfig {
    pub database_url: String,
    pub backend: Option<DatabaseBackend>,
    pub retention_days: u32,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub auto_migrate: bool,
    pub metrics_snapshot_interval_secs: u64,
}
```
