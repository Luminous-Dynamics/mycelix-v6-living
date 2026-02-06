-- Mycelix Living Protocol Database Schema (PostgreSQL)
-- Version: 001_initial
-- Description: Initial schema for cycle history, phase transitions, metrics, and events

-- ============================================================================
-- Cycle History Table
-- Stores overall cycle information
-- ============================================================================
CREATE TABLE IF NOT EXISTS cycle_history (
    id BIGSERIAL PRIMARY KEY,
    cycle_number BIGINT NOT NULL UNIQUE,
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,  -- NULL if cycle is ongoing
    total_transitions INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cycle_history_cycle_number ON cycle_history(cycle_number);
CREATE INDEX IF NOT EXISTS idx_cycle_history_started_at ON cycle_history(started_at);

-- ============================================================================
-- Phase Transitions Table
-- Records each phase transition with metrics snapshot
-- ============================================================================
CREATE TABLE IF NOT EXISTS phase_transitions (
    id BIGSERIAL PRIMARY KEY,
    from_phase TEXT NOT NULL,
    to_phase TEXT NOT NULL,
    cycle_number BIGINT NOT NULL,
    transitioned_at TIMESTAMPTZ NOT NULL,
    metrics_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_phase_transitions_cycle
        FOREIGN KEY (cycle_number) REFERENCES cycle_history(cycle_number)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_phase_transitions_cycle ON phase_transitions(cycle_number);
CREATE INDEX IF NOT EXISTS idx_phase_transitions_from ON phase_transitions(from_phase);
CREATE INDEX IF NOT EXISTS idx_phase_transitions_to ON phase_transitions(to_phase);
CREATE INDEX IF NOT EXISTS idx_phase_transitions_at ON phase_transitions(transitioned_at);

-- GIN index for JSONB queries on metrics
CREATE INDEX IF NOT EXISTS idx_phase_transitions_metrics ON phase_transitions USING GIN (metrics_json);

-- ============================================================================
-- Metrics Snapshots Table
-- Periodic snapshots of server and cycle metrics
-- ============================================================================
CREATE TABLE IF NOT EXISTS metrics_snapshots (
    id BIGSERIAL PRIMARY KEY,
    cycle_number BIGINT NOT NULL,
    phase TEXT NOT NULL,

    -- Server metrics
    active_connections BIGINT NOT NULL DEFAULT 0,
    total_connections BIGINT NOT NULL DEFAULT 0,
    messages_received BIGINT NOT NULL DEFAULT 0,
    messages_sent BIGINT NOT NULL DEFAULT 0,

    -- Cycle/phase metrics
    spectral_k DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    mean_metabolic_trust DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    active_wounds BIGINT NOT NULL DEFAULT 0,
    composting_entities BIGINT NOT NULL DEFAULT 0,
    liminal_entities BIGINT NOT NULL DEFAULT 0,
    entangled_pairs BIGINT NOT NULL DEFAULT 0,
    held_uncertainties BIGINT NOT NULL DEFAULT 0,

    snapshot_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_metrics_snapshots_cycle
        FOREIGN KEY (cycle_number) REFERENCES cycle_history(cycle_number)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_cycle ON metrics_snapshots(cycle_number);
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_phase ON metrics_snapshots(phase);
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_at ON metrics_snapshots(snapshot_at);

-- BRIN index for time-series queries (efficient for large datasets)
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_at_brin ON metrics_snapshots USING BRIN (snapshot_at);

-- ============================================================================
-- Events Table
-- Stores all Living Protocol events for replay/analysis
-- ============================================================================
CREATE TABLE IF NOT EXISTS events (
    id BIGSERIAL PRIMARY KEY,
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    cycle_number BIGINT NOT NULL,
    phase TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_events_cycle
        FOREIGN KEY (cycle_number) REFERENCES cycle_history(cycle_number)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_cycle ON events(cycle_number);
CREATE INDEX IF NOT EXISTS idx_events_phase ON events(phase);
CREATE INDEX IF NOT EXISTS idx_events_at ON events(created_at);

-- GIN index for JSONB queries on event data
CREATE INDEX IF NOT EXISTS idx_events_data ON events USING GIN (event_data);

-- BRIN index for time-series queries
CREATE INDEX IF NOT EXISTS idx_events_at_brin ON events USING BRIN (created_at);

-- ============================================================================
-- Schema Migrations Table
-- Tracks applied migrations
-- ============================================================================
CREATE TABLE IF NOT EXISTS schema_migrations (
    id SERIAL PRIMARY KEY,
    version TEXT NOT NULL UNIQUE,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    description TEXT
);

-- Record this migration
INSERT INTO schema_migrations (version, description)
VALUES ('001_initial', 'Initial schema for cycle history, phase transitions, metrics, and events')
ON CONFLICT (version) DO NOTHING;

-- ============================================================================
-- Views for common queries
-- ============================================================================

-- Latest cycle state
CREATE OR REPLACE VIEW v_current_cycle AS
SELECT
    ch.cycle_number,
    ch.started_at,
    pt.to_phase as current_phase,
    pt.transitioned_at as phase_started_at,
    (SELECT COUNT(*) FROM phase_transitions WHERE cycle_number = ch.cycle_number) as transitions_count
FROM cycle_history ch
LEFT JOIN phase_transitions pt ON pt.cycle_number = ch.cycle_number
WHERE ch.ended_at IS NULL
ORDER BY pt.transitioned_at DESC
LIMIT 1;

-- Daily metrics summary
CREATE OR REPLACE VIEW v_daily_metrics AS
SELECT
    DATE(snapshot_at) as date,
    COUNT(*) as snapshot_count,
    AVG(active_connections) as avg_connections,
    MAX(active_connections) as max_connections,
    SUM(messages_received) as total_received,
    SUM(messages_sent) as total_sent,
    AVG(spectral_k) as avg_spectral_k,
    AVG(mean_metabolic_trust) as avg_metabolic_trust
FROM metrics_snapshots
GROUP BY DATE(snapshot_at)
ORDER BY date DESC;

-- Phase duration summary
CREATE OR REPLACE VIEW v_phase_durations AS
SELECT
    pt1.from_phase as phase,
    pt1.cycle_number,
    pt1.transitioned_at as ended_at,
    LAG(pt1.transitioned_at) OVER (
        PARTITION BY pt1.cycle_number
        ORDER BY pt1.transitioned_at
    ) as started_at,
    EXTRACT(EPOCH FROM (
        pt1.transitioned_at - LAG(pt1.transitioned_at) OVER (
            PARTITION BY pt1.cycle_number
            ORDER BY pt1.transitioned_at
        )
    )) / 3600.0 as duration_hours
FROM phase_transitions pt1
ORDER BY pt1.transitioned_at DESC;

-- ============================================================================
-- Partitioning functions (for high-volume deployments)
-- ============================================================================

-- Function to create monthly partitions for metrics_snapshots
CREATE OR REPLACE FUNCTION create_metrics_partition(partition_date DATE)
RETURNS VOID AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    partition_name := 'metrics_snapshots_' || TO_CHAR(partition_date, 'YYYY_MM');
    start_date := DATE_TRUNC('month', partition_date);
    end_date := start_date + INTERVAL '1 month';

    EXECUTE FORMAT(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF metrics_snapshots_partitioned
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );
END;
$$ LANGUAGE plpgsql;

-- Function to create monthly partitions for events
CREATE OR REPLACE FUNCTION create_events_partition(partition_date DATE)
RETURNS VOID AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    partition_name := 'events_' || TO_CHAR(partition_date, 'YYYY_MM');
    start_date := DATE_TRUNC('month', partition_date);
    end_date := start_date + INTERVAL '1 month';

    EXECUTE FORMAT(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF events_partitioned
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Cleanup function for data retention
-- ============================================================================
CREATE OR REPLACE FUNCTION cleanup_old_data(retention_days INTEGER)
RETURNS TABLE(table_name TEXT, deleted_count BIGINT) AS $$
DECLARE
    cutoff_date TIMESTAMPTZ;
    deleted BIGINT;
BEGIN
    cutoff_date := NOW() - (retention_days || ' days')::INTERVAL;

    -- Clean phase_transitions
    DELETE FROM phase_transitions WHERE transitioned_at < cutoff_date;
    GET DIAGNOSTICS deleted = ROW_COUNT;
    table_name := 'phase_transitions';
    deleted_count := deleted;
    RETURN NEXT;

    -- Clean metrics_snapshots
    DELETE FROM metrics_snapshots WHERE snapshot_at < cutoff_date;
    GET DIAGNOSTICS deleted = ROW_COUNT;
    table_name := 'metrics_snapshots';
    deleted_count := deleted;
    RETURN NEXT;

    -- Clean events
    DELETE FROM events WHERE created_at < cutoff_date;
    GET DIAGNOSTICS deleted = ROW_COUNT;
    table_name := 'events';
    deleted_count := deleted;
    RETURN NEXT;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Statistics and analytics functions
-- ============================================================================

-- Get phase transition statistics
CREATE OR REPLACE FUNCTION get_phase_stats(from_date TIMESTAMPTZ, to_date TIMESTAMPTZ)
RETURNS TABLE(
    phase TEXT,
    transition_count BIGINT,
    avg_duration_hours DOUBLE PRECISION,
    min_duration_hours DOUBLE PRECISION,
    max_duration_hours DOUBLE PRECISION
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        v.phase,
        COUNT(*)::BIGINT as transition_count,
        AVG(v.duration_hours) as avg_duration_hours,
        MIN(v.duration_hours) as min_duration_hours,
        MAX(v.duration_hours) as max_duration_hours
    FROM v_phase_durations v
    WHERE v.ended_at >= from_date AND v.ended_at <= to_date
    GROUP BY v.phase
    ORDER BY transition_count DESC;
END;
$$ LANGUAGE plpgsql;

-- Get metrics trend for a specific metric
CREATE OR REPLACE FUNCTION get_metrics_trend(
    metric_name TEXT,
    from_date TIMESTAMPTZ,
    to_date TIMESTAMPTZ,
    bucket_hours INTEGER DEFAULT 24
)
RETURNS TABLE(
    bucket_start TIMESTAMPTZ,
    avg_value DOUBLE PRECISION,
    min_value DOUBLE PRECISION,
    max_value DOUBLE PRECISION,
    sample_count BIGINT
) AS $$
BEGIN
    RETURN QUERY
    EXECUTE FORMAT(
        'SELECT
            DATE_TRUNC(''hour'', snapshot_at) -
                (EXTRACT(HOUR FROM snapshot_at)::INTEGER %% %s) * INTERVAL ''1 hour'' as bucket_start,
            AVG(%I)::DOUBLE PRECISION as avg_value,
            MIN(%I)::DOUBLE PRECISION as min_value,
            MAX(%I)::DOUBLE PRECISION as max_value,
            COUNT(*)::BIGINT as sample_count
        FROM metrics_snapshots
        WHERE snapshot_at >= %L AND snapshot_at <= %L
        GROUP BY bucket_start
        ORDER BY bucket_start',
        bucket_hours, metric_name, metric_name, metric_name, from_date, to_date
    );
END;
$$ LANGUAGE plpgsql;
