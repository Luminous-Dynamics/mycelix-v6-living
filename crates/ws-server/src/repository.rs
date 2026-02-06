//! Repository implementations for persisting cycle history and metrics.
//!
//! This module provides trait-based repository access for database operations,
//! with implementations for SQLite and PostgreSQL backends.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use tracing::{debug, info};

use living_core::{CyclePhase, PhaseMetrics, PhaseTransition};

use crate::persistence::{
    DatabasePool, PersistenceConfig, PersistenceError, PersistenceResult,
    StoredEvent, StoredMetricsSnapshot, StoredPhaseTransition,
};
use crate::server::ServerMetrics;

/// Repository trait for cycle persistence operations.
#[async_trait]
pub trait CycleRepository: Send + Sync {
    /// Save a phase transition to the database.
    async fn save_transition(&self, transition: &PhaseTransition) -> PersistenceResult<i64>;

    /// Get transition history for a given time range.
    async fn get_history(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: Option<u32>,
    ) -> PersistenceResult<Vec<StoredPhaseTransition>>;

    /// Get the most recent transitions.
    async fn get_recent_transitions(
        &self,
        limit: u32,
    ) -> PersistenceResult<Vec<StoredPhaseTransition>>;

    /// Save a metrics snapshot.
    async fn save_metrics(
        &self,
        cycle_number: u64,
        phase: CyclePhase,
        server_metrics: &ServerMetrics,
        phase_metrics: &PhaseMetrics,
    ) -> PersistenceResult<i64>;

    /// Get metrics snapshots for a time range.
    async fn get_metrics_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> PersistenceResult<Vec<StoredMetricsSnapshot>>;

    /// Get the latest metrics snapshot.
    async fn get_latest_metrics(&self) -> PersistenceResult<Option<StoredMetricsSnapshot>>;

    /// Save an event to the database.
    async fn save_event(
        &self,
        event_type: &str,
        event_data: &str,
        cycle_number: u64,
        phase: CyclePhase,
    ) -> PersistenceResult<i64>;

    /// Get events for a time range.
    async fn get_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        event_type: Option<&str>,
    ) -> PersistenceResult<Vec<StoredEvent>>;

    /// Clean up old data based on retention policy.
    async fn cleanup_old_data(&self, retention_days: u32) -> PersistenceResult<u64>;

    /// Get the last saved cycle number.
    async fn get_last_cycle_number(&self) -> PersistenceResult<Option<u64>>;

    /// Get the last saved phase.
    async fn get_last_phase(&self) -> PersistenceResult<Option<CyclePhase>>;
}

/// SQLite implementation of the cycle repository.
#[cfg(feature = "sqlite")]
pub struct SqliteRepository {
    pool: sqlx::sqlite::SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SqliteRepository {
    /// Create a new SQLite repository from a database pool.
    pub fn new(pool: sqlx::sqlite::SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new SQLite repository from a configuration.
    pub async fn from_config(config: &PersistenceConfig) -> PersistenceResult<Self> {
        let db_pool = DatabasePool::connect(config).await?;
        match db_pool {
            DatabasePool::Sqlite(pool) => Ok(Self::new(pool)),
            #[cfg(feature = "postgres")]
            _ => Err(PersistenceError::Configuration(
                "Expected SQLite configuration".to_string(),
            )),
        }
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl CycleRepository for SqliteRepository {
    async fn save_transition(&self, transition: &PhaseTransition) -> PersistenceResult<i64> {
        let from_phase = format!("{:?}", transition.from);
        let to_phase = format!("{:?}", transition.to);
        let metrics_json = serde_json::to_string(&transition.metrics)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let result = sqlx::query(
            r#"
            INSERT INTO phase_transitions (from_phase, to_phase, cycle_number, transitioned_at, metrics_json)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&from_phase)
        .bind(&to_phase)
        .bind(transition.cycle_number as i64)
        .bind(transition.transitioned_at)
        .bind(&metrics_json)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();
        debug!(id = id, from = %from_phase, to = %to_phase, "Saved phase transition");
        Ok(id)
    }

    async fn get_history(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: Option<u32>,
    ) -> PersistenceResult<Vec<StoredPhaseTransition>> {
        let limit = limit.unwrap_or(1000) as i64;

        let rows = sqlx::query_as::<_, (i64, String, String, i64, DateTime<Utc>, String, DateTime<Utc>)>(
            r#"
            SELECT id, from_phase, to_phase, cycle_number, transitioned_at, metrics_json, created_at
            FROM phase_transitions
            WHERE transitioned_at >= ? AND transitioned_at <= ?
            ORDER BY transitioned_at DESC
            LIMIT ?
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, from_phase, to_phase, cycle_number, transitioned_at, metrics_json, created_at)| {
                StoredPhaseTransition {
                    id,
                    from_phase,
                    to_phase,
                    cycle_number,
                    transitioned_at,
                    metrics_json,
                    created_at,
                }
            })
            .collect())
    }

    async fn get_recent_transitions(
        &self,
        limit: u32,
    ) -> PersistenceResult<Vec<StoredPhaseTransition>> {
        let rows = sqlx::query_as::<_, (i64, String, String, i64, DateTime<Utc>, String, DateTime<Utc>)>(
            r#"
            SELECT id, from_phase, to_phase, cycle_number, transitioned_at, metrics_json, created_at
            FROM phase_transitions
            ORDER BY transitioned_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, from_phase, to_phase, cycle_number, transitioned_at, metrics_json, created_at)| {
                StoredPhaseTransition {
                    id,
                    from_phase,
                    to_phase,
                    cycle_number,
                    transitioned_at,
                    metrics_json,
                    created_at,
                }
            })
            .collect())
    }

    async fn save_metrics(
        &self,
        cycle_number: u64,
        phase: CyclePhase,
        server_metrics: &ServerMetrics,
        phase_metrics: &PhaseMetrics,
    ) -> PersistenceResult<i64> {
        let phase_str = format!("{:?}", phase);

        let result = sqlx::query(
            r#"
            INSERT INTO metrics_snapshots (
                cycle_number, phase, active_connections, total_connections,
                messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                active_wounds, composting_entities, liminal_entities,
                entangled_pairs, held_uncertainties
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(cycle_number as i64)
        .bind(&phase_str)
        .bind(server_metrics.active_connections as i64)
        .bind(server_metrics.total_connections as i64)
        .bind(server_metrics.messages_received as i64)
        .bind(server_metrics.messages_sent as i64)
        .bind(phase_metrics.spectral_k)
        .bind(phase_metrics.mean_metabolic_trust)
        .bind(phase_metrics.active_wounds as i64)
        .bind(phase_metrics.composting_entities as i64)
        .bind(phase_metrics.liminal_entities as i64)
        .bind(phase_metrics.entangled_pairs as i64)
        .bind(phase_metrics.held_uncertainties as i64)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();
        debug!(id = id, cycle = cycle_number, phase = %phase_str, "Saved metrics snapshot");
        Ok(id)
    }

    async fn get_metrics_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> PersistenceResult<Vec<StoredMetricsSnapshot>> {
        let rows = sqlx::query_as::<_, (i64, i64, String, i64, i64, i64, i64, f64, f64, i64, i64, i64, i64, i64, DateTime<Utc>)>(
            r#"
            SELECT id, cycle_number, phase, active_connections, total_connections,
                   messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                   active_wounds, composting_entities, liminal_entities,
                   entangled_pairs, held_uncertainties, snapshot_at
            FROM metrics_snapshots
            WHERE snapshot_at >= ? AND snapshot_at <= ?
            ORDER BY snapshot_at DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, cycle_number, phase, active_connections, total_connections,
                   messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                   active_wounds, composting_entities, liminal_entities,
                   entangled_pairs, held_uncertainties, snapshot_at)| {
                StoredMetricsSnapshot {
                    id,
                    cycle_number,
                    phase,
                    active_connections,
                    total_connections,
                    messages_received,
                    messages_sent,
                    spectral_k,
                    mean_metabolic_trust,
                    active_wounds,
                    composting_entities,
                    liminal_entities,
                    entangled_pairs,
                    held_uncertainties,
                    snapshot_at,
                }
            })
            .collect())
    }

    async fn get_latest_metrics(&self) -> PersistenceResult<Option<StoredMetricsSnapshot>> {
        let row = sqlx::query_as::<_, (i64, i64, String, i64, i64, i64, i64, f64, f64, i64, i64, i64, i64, i64, DateTime<Utc>)>(
            r#"
            SELECT id, cycle_number, phase, active_connections, total_connections,
                   messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                   active_wounds, composting_entities, liminal_entities,
                   entangled_pairs, held_uncertainties, snapshot_at
            FROM metrics_snapshots
            ORDER BY snapshot_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, cycle_number, phase, active_connections, total_connections,
                   messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                   active_wounds, composting_entities, liminal_entities,
                   entangled_pairs, held_uncertainties, snapshot_at)| {
            StoredMetricsSnapshot {
                id,
                cycle_number,
                phase,
                active_connections,
                total_connections,
                messages_received,
                messages_sent,
                spectral_k,
                mean_metabolic_trust,
                active_wounds,
                composting_entities,
                liminal_entities,
                entangled_pairs,
                held_uncertainties,
                snapshot_at,
            }
        }))
    }

    async fn save_event(
        &self,
        event_type: &str,
        event_data: &str,
        cycle_number: u64,
        phase: CyclePhase,
    ) -> PersistenceResult<i64> {
        let phase_str = format!("{:?}", phase);

        let result = sqlx::query(
            r#"
            INSERT INTO events (event_type, event_data, cycle_number, phase)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(event_type)
        .bind(event_data)
        .bind(cycle_number as i64)
        .bind(&phase_str)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn get_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        event_type: Option<&str>,
    ) -> PersistenceResult<Vec<StoredEvent>> {
        let rows = if let Some(event_type) = event_type {
            sqlx::query_as::<_, (i64, String, String, i64, String, DateTime<Utc>)>(
                r#"
                SELECT id, event_type, event_data, cycle_number, phase, created_at
                FROM events
                WHERE created_at >= ? AND created_at <= ? AND event_type = ?
                ORDER BY created_at DESC
                "#,
            )
            .bind(from)
            .bind(to)
            .bind(event_type)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (i64, String, String, i64, String, DateTime<Utc>)>(
                r#"
                SELECT id, event_type, event_data, cycle_number, phase, created_at
                FROM events
                WHERE created_at >= ? AND created_at <= ?
                ORDER BY created_at DESC
                "#,
            )
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|(id, event_type, event_data, cycle_number, phase, created_at)| {
                StoredEvent {
                    id,
                    event_type,
                    event_data,
                    cycle_number,
                    phase,
                    created_at,
                }
            })
            .collect())
    }

    async fn cleanup_old_data(&self, retention_days: u32) -> PersistenceResult<u64> {
        let cutoff = Utc::now() - Duration::days(retention_days as i64);
        let mut total_deleted = 0u64;

        // Clean up phase transitions
        let result = sqlx::query("DELETE FROM phase_transitions WHERE transitioned_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        total_deleted += result.rows_affected();

        // Clean up metrics snapshots
        let result = sqlx::query("DELETE FROM metrics_snapshots WHERE snapshot_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        total_deleted += result.rows_affected();

        // Clean up events
        let result = sqlx::query("DELETE FROM events WHERE created_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        total_deleted += result.rows_affected();

        if total_deleted > 0 {
            info!(
                deleted = total_deleted,
                retention_days = retention_days,
                "Cleaned up old data"
            );
        }

        Ok(total_deleted)
    }

    async fn get_last_cycle_number(&self) -> PersistenceResult<Option<u64>> {
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            SELECT cycle_number FROM phase_transitions
            ORDER BY transitioned_at DESC LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(n,)| n as u64))
    }

    async fn get_last_phase(&self) -> PersistenceResult<Option<CyclePhase>> {
        let row = sqlx::query_as::<_, (String,)>(
            r#"
            SELECT to_phase FROM phase_transitions
            ORDER BY transitioned_at DESC LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|(phase_str,)| parse_cycle_phase(&phase_str)))
    }
}

/// PostgreSQL implementation of the cycle repository.
#[cfg(feature = "postgres")]
pub struct PostgresRepository {
    pool: sqlx::postgres::PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresRepository {
    /// Create a new PostgreSQL repository from a database pool.
    pub fn new(pool: sqlx::postgres::PgPool) -> Self {
        Self { pool }
    }

    /// Create a new PostgreSQL repository from a configuration.
    pub async fn from_config(config: &PersistenceConfig) -> PersistenceResult<Self> {
        let db_pool = DatabasePool::connect(config).await?;
        match db_pool {
            DatabasePool::Postgres(pool) => Ok(Self::new(pool)),
            #[cfg(feature = "sqlite")]
            _ => Err(PersistenceError::Configuration(
                "Expected PostgreSQL configuration".to_string(),
            )),
        }
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl CycleRepository for PostgresRepository {
    async fn save_transition(&self, transition: &PhaseTransition) -> PersistenceResult<i64> {
        let from_phase = format!("{:?}", transition.from);
        let to_phase = format!("{:?}", transition.to);
        let metrics_json = serde_json::to_string(&transition.metrics)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO phase_transitions (from_phase, to_phase, cycle_number, transitioned_at, metrics_json)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(&from_phase)
        .bind(&to_phase)
        .bind(transition.cycle_number as i64)
        .bind(transition.transitioned_at)
        .bind(&metrics_json)
        .fetch_one(&self.pool)
        .await?;

        debug!(id = row.0, from = %from_phase, to = %to_phase, "Saved phase transition");
        Ok(row.0)
    }

    async fn get_history(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: Option<u32>,
    ) -> PersistenceResult<Vec<StoredPhaseTransition>> {
        let limit = limit.unwrap_or(1000) as i64;

        let rows = sqlx::query_as::<_, (i64, String, String, i64, DateTime<Utc>, String, DateTime<Utc>)>(
            r#"
            SELECT id, from_phase, to_phase, cycle_number, transitioned_at, metrics_json, created_at
            FROM phase_transitions
            WHERE transitioned_at >= $1 AND transitioned_at <= $2
            ORDER BY transitioned_at DESC
            LIMIT $3
            "#,
        )
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, from_phase, to_phase, cycle_number, transitioned_at, metrics_json, created_at)| {
                StoredPhaseTransition {
                    id,
                    from_phase,
                    to_phase,
                    cycle_number,
                    transitioned_at,
                    metrics_json,
                    created_at,
                }
            })
            .collect())
    }

    async fn get_recent_transitions(
        &self,
        limit: u32,
    ) -> PersistenceResult<Vec<StoredPhaseTransition>> {
        let rows = sqlx::query_as::<_, (i64, String, String, i64, DateTime<Utc>, String, DateTime<Utc>)>(
            r#"
            SELECT id, from_phase, to_phase, cycle_number, transitioned_at, metrics_json, created_at
            FROM phase_transitions
            ORDER BY transitioned_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, from_phase, to_phase, cycle_number, transitioned_at, metrics_json, created_at)| {
                StoredPhaseTransition {
                    id,
                    from_phase,
                    to_phase,
                    cycle_number,
                    transitioned_at,
                    metrics_json,
                    created_at,
                }
            })
            .collect())
    }

    async fn save_metrics(
        &self,
        cycle_number: u64,
        phase: CyclePhase,
        server_metrics: &ServerMetrics,
        phase_metrics: &PhaseMetrics,
    ) -> PersistenceResult<i64> {
        let phase_str = format!("{:?}", phase);

        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO metrics_snapshots (
                cycle_number, phase, active_connections, total_connections,
                messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                active_wounds, composting_entities, liminal_entities,
                entangled_pairs, held_uncertainties
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id
            "#,
        )
        .bind(cycle_number as i64)
        .bind(&phase_str)
        .bind(server_metrics.active_connections as i64)
        .bind(server_metrics.total_connections as i64)
        .bind(server_metrics.messages_received as i64)
        .bind(server_metrics.messages_sent as i64)
        .bind(phase_metrics.spectral_k)
        .bind(phase_metrics.mean_metabolic_trust)
        .bind(phase_metrics.active_wounds as i64)
        .bind(phase_metrics.composting_entities as i64)
        .bind(phase_metrics.liminal_entities as i64)
        .bind(phase_metrics.entangled_pairs as i64)
        .bind(phase_metrics.held_uncertainties as i64)
        .fetch_one(&self.pool)
        .await?;

        debug!(id = row.0, cycle = cycle_number, phase = %phase_str, "Saved metrics snapshot");
        Ok(row.0)
    }

    async fn get_metrics_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> PersistenceResult<Vec<StoredMetricsSnapshot>> {
        let rows = sqlx::query_as::<_, (i64, i64, String, i64, i64, i64, i64, f64, f64, i64, i64, i64, i64, i64, DateTime<Utc>)>(
            r#"
            SELECT id, cycle_number, phase, active_connections, total_connections,
                   messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                   active_wounds, composting_entities, liminal_entities,
                   entangled_pairs, held_uncertainties, snapshot_at
            FROM metrics_snapshots
            WHERE snapshot_at >= $1 AND snapshot_at <= $2
            ORDER BY snapshot_at DESC
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, cycle_number, phase, active_connections, total_connections,
                   messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                   active_wounds, composting_entities, liminal_entities,
                   entangled_pairs, held_uncertainties, snapshot_at)| {
                StoredMetricsSnapshot {
                    id,
                    cycle_number,
                    phase,
                    active_connections,
                    total_connections,
                    messages_received,
                    messages_sent,
                    spectral_k,
                    mean_metabolic_trust,
                    active_wounds,
                    composting_entities,
                    liminal_entities,
                    entangled_pairs,
                    held_uncertainties,
                    snapshot_at,
                }
            })
            .collect())
    }

    async fn get_latest_metrics(&self) -> PersistenceResult<Option<StoredMetricsSnapshot>> {
        let row = sqlx::query_as::<_, (i64, i64, String, i64, i64, i64, i64, f64, f64, i64, i64, i64, i64, i64, DateTime<Utc>)>(
            r#"
            SELECT id, cycle_number, phase, active_connections, total_connections,
                   messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                   active_wounds, composting_entities, liminal_entities,
                   entangled_pairs, held_uncertainties, snapshot_at
            FROM metrics_snapshots
            ORDER BY snapshot_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, cycle_number, phase, active_connections, total_connections,
                   messages_received, messages_sent, spectral_k, mean_metabolic_trust,
                   active_wounds, composting_entities, liminal_entities,
                   entangled_pairs, held_uncertainties, snapshot_at)| {
            StoredMetricsSnapshot {
                id,
                cycle_number,
                phase,
                active_connections,
                total_connections,
                messages_received,
                messages_sent,
                spectral_k,
                mean_metabolic_trust,
                active_wounds,
                composting_entities,
                liminal_entities,
                entangled_pairs,
                held_uncertainties,
                snapshot_at,
            }
        }))
    }

    async fn save_event(
        &self,
        event_type: &str,
        event_data: &str,
        cycle_number: u64,
        phase: CyclePhase,
    ) -> PersistenceResult<i64> {
        let phase_str = format!("{:?}", phase);

        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO events (event_type, event_data, cycle_number, phase)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(event_type)
        .bind(event_data)
        .bind(cycle_number as i64)
        .bind(&phase_str)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    async fn get_events(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        event_type: Option<&str>,
    ) -> PersistenceResult<Vec<StoredEvent>> {
        let rows = if let Some(event_type) = event_type {
            sqlx::query_as::<_, (i64, String, String, i64, String, DateTime<Utc>)>(
                r#"
                SELECT id, event_type, event_data, cycle_number, phase, created_at
                FROM events
                WHERE created_at >= $1 AND created_at <= $2 AND event_type = $3
                ORDER BY created_at DESC
                "#,
            )
            .bind(from)
            .bind(to)
            .bind(event_type)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, (i64, String, String, i64, String, DateTime<Utc>)>(
                r#"
                SELECT id, event_type, event_data, cycle_number, phase, created_at
                FROM events
                WHERE created_at >= $1 AND created_at <= $2
                ORDER BY created_at DESC
                "#,
            )
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|(id, event_type, event_data, cycle_number, phase, created_at)| {
                StoredEvent {
                    id,
                    event_type,
                    event_data,
                    cycle_number,
                    phase,
                    created_at,
                }
            })
            .collect())
    }

    async fn cleanup_old_data(&self, retention_days: u32) -> PersistenceResult<u64> {
        let cutoff = Utc::now() - Duration::days(retention_days as i64);
        let mut total_deleted = 0u64;

        // Clean up phase transitions
        let result = sqlx::query("DELETE FROM phase_transitions WHERE transitioned_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        total_deleted += result.rows_affected();

        // Clean up metrics snapshots
        let result = sqlx::query("DELETE FROM metrics_snapshots WHERE snapshot_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        total_deleted += result.rows_affected();

        // Clean up events
        let result = sqlx::query("DELETE FROM events WHERE created_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        total_deleted += result.rows_affected();

        if total_deleted > 0 {
            info!(
                deleted = total_deleted,
                retention_days = retention_days,
                "Cleaned up old data"
            );
        }

        Ok(total_deleted)
    }

    async fn get_last_cycle_number(&self) -> PersistenceResult<Option<u64>> {
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            SELECT cycle_number FROM phase_transitions
            ORDER BY transitioned_at DESC LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(n,)| n as u64))
    }

    async fn get_last_phase(&self) -> PersistenceResult<Option<CyclePhase>> {
        let row = sqlx::query_as::<_, (String,)>(
            r#"
            SELECT to_phase FROM phase_transitions
            ORDER BY transitioned_at DESC LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|(phase_str,)| parse_cycle_phase(&phase_str)))
    }
}

/// Parse a cycle phase from a string.
fn parse_cycle_phase(s: &str) -> Option<CyclePhase> {
    match s {
        "Shadow" => Some(CyclePhase::Shadow),
        "Composting" => Some(CyclePhase::Composting),
        "Liminal" => Some(CyclePhase::Liminal),
        "NegativeCapability" => Some(CyclePhase::NegativeCapability),
        "Eros" => Some(CyclePhase::Eros),
        "CoCreation" => Some(CyclePhase::CoCreation),
        "Beauty" => Some(CyclePhase::Beauty),
        "EmergentPersonhood" => Some(CyclePhase::EmergentPersonhood),
        "Kenosis" => Some(CyclePhase::Kenosis),
        _ => None,
    }
}

/// Create a repository from configuration.
pub async fn create_repository(
    config: &PersistenceConfig,
) -> PersistenceResult<Arc<dyn CycleRepository>> {
    let pool = DatabasePool::connect(config).await?;

    if config.auto_migrate {
        pool.migrate().await?;
    }

    match pool {
        #[cfg(feature = "sqlite")]
        DatabasePool::Sqlite(sqlite_pool) => {
            Ok(Arc::new(SqliteRepository::new(sqlite_pool)))
        }
        #[cfg(feature = "postgres")]
        DatabasePool::Postgres(pg_pool) => {
            Ok(Arc::new(PostgresRepository::new(pg_pool)))
        }
        #[cfg(not(any(feature = "sqlite", feature = "postgres")))]
        _ => Err(PersistenceError::NotInitialized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cycle_phase() {
        assert_eq!(parse_cycle_phase("Shadow"), Some(CyclePhase::Shadow));
        assert_eq!(parse_cycle_phase("Composting"), Some(CyclePhase::Composting));
        assert_eq!(parse_cycle_phase("Liminal"), Some(CyclePhase::Liminal));
        assert_eq!(parse_cycle_phase("NegativeCapability"), Some(CyclePhase::NegativeCapability));
        assert_eq!(parse_cycle_phase("Eros"), Some(CyclePhase::Eros));
        assert_eq!(parse_cycle_phase("CoCreation"), Some(CyclePhase::CoCreation));
        assert_eq!(parse_cycle_phase("Beauty"), Some(CyclePhase::Beauty));
        assert_eq!(parse_cycle_phase("EmergentPersonhood"), Some(CyclePhase::EmergentPersonhood));
        assert_eq!(parse_cycle_phase("Kenosis"), Some(CyclePhase::Kenosis));
        assert_eq!(parse_cycle_phase("Invalid"), None);
    }
}
