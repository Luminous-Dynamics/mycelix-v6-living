-- Mycelix Living Protocol Database Schema (SQLite)
-- Version: 001_initial
-- Description: Initial schema for cycle history, phase transitions, metrics, and events

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- ============================================================================
-- Cycle History Table
-- Stores overall cycle information
-- ============================================================================
CREATE TABLE IF NOT EXISTS cycle_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cycle_number INTEGER NOT NULL UNIQUE,
    started_at TEXT NOT NULL,  -- ISO 8601 datetime
    ended_at TEXT,             -- NULL if cycle is ongoing
    total_transitions INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_cycle_history_cycle_number ON cycle_history(cycle_number);
CREATE INDEX IF NOT EXISTS idx_cycle_history_started_at ON cycle_history(started_at);

-- ============================================================================
-- Phase Transitions Table
-- Records each phase transition with metrics snapshot
-- ============================================================================
CREATE TABLE IF NOT EXISTS phase_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_phase TEXT NOT NULL,
    to_phase TEXT NOT NULL,
    cycle_number INTEGER NOT NULL,
    transitioned_at TEXT NOT NULL,  -- ISO 8601 datetime
    metrics_json TEXT NOT NULL,     -- JSON blob of PhaseMetrics
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (cycle_number) REFERENCES cycle_history(cycle_number)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_phase_transitions_cycle ON phase_transitions(cycle_number);
CREATE INDEX IF NOT EXISTS idx_phase_transitions_from ON phase_transitions(from_phase);
CREATE INDEX IF NOT EXISTS idx_phase_transitions_to ON phase_transitions(to_phase);
CREATE INDEX IF NOT EXISTS idx_phase_transitions_at ON phase_transitions(transitioned_at);

-- ============================================================================
-- Metrics Snapshots Table
-- Periodic snapshots of server and cycle metrics
-- ============================================================================
CREATE TABLE IF NOT EXISTS metrics_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cycle_number INTEGER NOT NULL,
    phase TEXT NOT NULL,

    -- Server metrics
    active_connections INTEGER NOT NULL DEFAULT 0,
    total_connections INTEGER NOT NULL DEFAULT 0,
    messages_received INTEGER NOT NULL DEFAULT 0,
    messages_sent INTEGER NOT NULL DEFAULT 0,

    -- Cycle/phase metrics
    spectral_k REAL NOT NULL DEFAULT 0.0,
    mean_metabolic_trust REAL NOT NULL DEFAULT 0.0,
    active_wounds INTEGER NOT NULL DEFAULT 0,
    composting_entities INTEGER NOT NULL DEFAULT 0,
    liminal_entities INTEGER NOT NULL DEFAULT 0,
    entangled_pairs INTEGER NOT NULL DEFAULT 0,
    held_uncertainties INTEGER NOT NULL DEFAULT 0,

    snapshot_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (cycle_number) REFERENCES cycle_history(cycle_number)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_cycle ON metrics_snapshots(cycle_number);
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_phase ON metrics_snapshots(phase);
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_at ON metrics_snapshots(snapshot_at);

-- ============================================================================
-- Events Table
-- Stores all Living Protocol events for replay/analysis
-- ============================================================================
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,  -- JSON serialized event
    cycle_number INTEGER NOT NULL,
    phase TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (cycle_number) REFERENCES cycle_history(cycle_number)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_cycle ON events(cycle_number);
CREATE INDEX IF NOT EXISTS idx_events_phase ON events(phase);
CREATE INDEX IF NOT EXISTS idx_events_at ON events(created_at);

-- ============================================================================
-- Schema Migrations Table
-- Tracks applied migrations
-- ============================================================================
CREATE TABLE IF NOT EXISTS schema_migrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    version TEXT NOT NULL UNIQUE,
    applied_at TEXT NOT NULL DEFAULT (datetime('now')),
    description TEXT
);

-- Record this migration
INSERT OR IGNORE INTO schema_migrations (version, description)
VALUES ('001_initial', 'Initial schema for cycle history, phase transitions, metrics, and events');

-- ============================================================================
-- Views for common queries
-- ============================================================================

-- Latest cycle state
CREATE VIEW IF NOT EXISTS v_current_cycle AS
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
CREATE VIEW IF NOT EXISTS v_daily_metrics AS
SELECT
    date(snapshot_at) as date,
    COUNT(*) as snapshot_count,
    AVG(active_connections) as avg_connections,
    MAX(active_connections) as max_connections,
    SUM(messages_received) as total_received,
    SUM(messages_sent) as total_sent,
    AVG(spectral_k) as avg_spectral_k,
    AVG(mean_metabolic_trust) as avg_metabolic_trust
FROM metrics_snapshots
GROUP BY date(snapshot_at)
ORDER BY date DESC;

-- Phase duration summary
CREATE VIEW IF NOT EXISTS v_phase_durations AS
SELECT
    pt1.from_phase as phase,
    pt1.cycle_number,
    pt1.transitioned_at as ended_at,
    (
        SELECT pt2.transitioned_at
        FROM phase_transitions pt2
        WHERE pt2.cycle_number = pt1.cycle_number
        AND pt2.to_phase = pt1.from_phase
        ORDER BY pt2.transitioned_at DESC
        LIMIT 1
    ) as started_at,
    ROUND(
        (julianday(pt1.transitioned_at) - julianday(
            (SELECT pt2.transitioned_at
             FROM phase_transitions pt2
             WHERE pt2.cycle_number = pt1.cycle_number
             AND pt2.to_phase = pt1.from_phase
             ORDER BY pt2.transitioned_at DESC
             LIMIT 1)
        )) * 24, 2
    ) as duration_hours
FROM phase_transitions pt1
ORDER BY pt1.transitioned_at DESC;
