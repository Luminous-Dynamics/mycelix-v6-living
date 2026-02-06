//! Persistence configuration and database connection management.
//!
//! This module provides the configuration and connection management for
//! persisting cycle history, phase transitions, and metrics to a database.
//! Supports SQLite (default) and PostgreSQL (feature-gated).

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "sqlite")]
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
#[cfg(feature = "postgres")]
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Errors that can occur during persistence operations.
#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("Database connection error: {0}")]
    Connection(String),

    #[error("Query execution error: {0}")]
    Query(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Database not initialized")]
    NotInitialized,

    #[cfg(any(feature = "sqlite", feature = "postgres"))]
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

/// Result type for persistence operations.
pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Database backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseBackend {
    /// SQLite database (default, file-based)
    Sqlite,
    /// PostgreSQL database (requires postgres feature)
    Postgres,
}

impl Default for DatabaseBackend {
    fn default() -> Self {
        Self::Sqlite
    }
}

/// Configuration for the persistence layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Database URL (e.g., "sqlite:./mycelix.db" or "postgres://user:pass@host/db")
    pub database_url: String,

    /// Database backend type (auto-detected from URL if not specified)
    pub backend: Option<DatabaseBackend>,

    /// Retention period for historical data in days
    pub retention_days: u32,

    /// Maximum number of database connections in the pool
    pub max_connections: u32,

    /// Minimum number of idle connections to maintain
    pub min_connections: u32,

    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,

    /// Idle connection timeout in seconds
    pub idle_timeout_secs: u64,

    /// Whether to run migrations automatically on startup
    pub auto_migrate: bool,

    /// Interval for metrics snapshots in seconds
    pub metrics_snapshot_interval_secs: u64,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:./mycelix.db".to_string(),
            backend: None,
            retention_days: 30,
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 30,
            idle_timeout_secs: 600,
            auto_migrate: true,
            metrics_snapshot_interval_secs: 60,
        }
    }
}

impl PersistenceConfig {
    /// Create a new configuration with the given database URL.
    pub fn new(database_url: &str) -> Self {
        let backend = Self::detect_backend(database_url);
        Self {
            database_url: database_url.to_string(),
            backend: Some(backend),
            ..Default::default()
        }
    }

    /// Create a SQLite configuration with the given path.
    pub fn sqlite(path: &str) -> Self {
        Self {
            database_url: format!("sqlite:{}", path),
            backend: Some(DatabaseBackend::Sqlite),
            ..Default::default()
        }
    }

    /// Create a PostgreSQL configuration with the given URL.
    #[cfg(feature = "postgres")]
    pub fn postgres(url: &str) -> Self {
        Self {
            database_url: url.to_string(),
            backend: Some(DatabaseBackend::Postgres),
            ..Default::default()
        }
    }

    /// Set the retention period in days.
    pub fn with_retention_days(mut self, days: u32) -> Self {
        self.retention_days = days;
        self
    }

    /// Set the metrics snapshot interval.
    pub fn with_metrics_interval(mut self, secs: u64) -> Self {
        self.metrics_snapshot_interval_secs = secs;
        self
    }

    /// Disable automatic migrations.
    pub fn without_auto_migrate(mut self) -> Self {
        self.auto_migrate = false;
        self
    }

    /// Detect the database backend from the URL.
    pub fn detect_backend(url: &str) -> DatabaseBackend {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            DatabaseBackend::Postgres
        } else {
            DatabaseBackend::Sqlite
        }
    }

    /// Get the effective backend.
    pub fn effective_backend(&self) -> DatabaseBackend {
        self.backend
            .unwrap_or_else(|| Self::detect_backend(&self.database_url))
    }

    /// Validate the configuration.
    pub fn validate(&self) -> PersistenceResult<()> {
        if self.database_url.is_empty() {
            return Err(PersistenceError::Configuration(
                "Database URL cannot be empty".to_string(),
            ));
        }

        let _backend = self.effective_backend();

        #[cfg(not(feature = "postgres"))]
        if _backend == DatabaseBackend::Postgres {
            return Err(PersistenceError::Configuration(
                "PostgreSQL support requires the 'postgres' feature".to_string(),
            ));
        }

        #[cfg(not(feature = "sqlite"))]
        if _backend == DatabaseBackend::Sqlite {
            return Err(PersistenceError::Configuration(
                "SQLite support requires the 'sqlite' feature".to_string(),
            ));
        }

        if self.retention_days == 0 {
            return Err(PersistenceError::Configuration(
                "Retention days must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// Database connection pool wrapper supporting multiple backends.
#[derive(Clone)]
pub enum DatabasePool {
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
    #[cfg(feature = "postgres")]
    Postgres(PgPool),
    /// Placeholder for when no database features are enabled
    #[cfg(not(any(feature = "sqlite", feature = "postgres")))]
    None,
}

impl DatabasePool {
    /// Create a new database connection pool from the configuration.
    pub async fn connect(config: &PersistenceConfig) -> PersistenceResult<Self> {
        config.validate()?;

        match config.effective_backend() {
            #[cfg(feature = "sqlite")]
            DatabaseBackend::Sqlite => {
                let url = config.database_url.strip_prefix("sqlite:").unwrap_or(&config.database_url);

                let options = SqliteConnectOptions::new()
                    .filename(url)
                    .create_if_missing(true);

                let pool = SqlitePoolOptions::new()
                    .max_connections(config.max_connections)
                    .min_connections(config.min_connections)
                    .acquire_timeout(Duration::from_secs(config.connect_timeout_secs))
                    .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
                    .connect_with(options)
                    .await?;

                Ok(DatabasePool::Sqlite(pool))
            }

            #[cfg(feature = "postgres")]
            DatabaseBackend::Postgres => {
                let pool = PgPoolOptions::new()
                    .max_connections(config.max_connections)
                    .min_connections(config.min_connections)
                    .acquire_timeout(Duration::from_secs(config.connect_timeout_secs))
                    .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
                    .connect(&config.database_url)
                    .await?;

                Ok(DatabasePool::Postgres(pool))
            }

            #[cfg(not(feature = "sqlite"))]
            DatabaseBackend::Sqlite => Err(PersistenceError::Configuration(
                "SQLite support requires the 'sqlite' feature".to_string(),
            )),

            #[cfg(not(feature = "postgres"))]
            DatabaseBackend::Postgres => Err(PersistenceError::Configuration(
                "PostgreSQL support requires the 'postgres' feature".to_string(),
            )),
        }
    }

    /// Run migrations for the database.
    pub async fn migrate(&self) -> PersistenceResult<()> {
        match self {
            #[cfg(feature = "sqlite")]
            DatabasePool::Sqlite(pool) => {
                Self::run_sqlite_migrations(pool).await
            }
            #[cfg(feature = "postgres")]
            DatabasePool::Postgres(pool) => {
                Self::run_postgres_migrations(pool).await
            }
            #[cfg(not(any(feature = "sqlite", feature = "postgres")))]
            DatabasePool::None => Err(PersistenceError::NotInitialized),
        }
    }

    #[cfg(feature = "sqlite")]
    async fn run_sqlite_migrations(pool: &SqlitePool) -> PersistenceResult<()> {
        // Run the initial schema migration
        sqlx::query(include_str!("../../../migrations/001_initial.sql"))
            .execute(pool)
            .await
            .map_err(|e| PersistenceError::Migration(e.to_string()))?;
        Ok(())
    }

    #[cfg(feature = "postgres")]
    async fn run_postgres_migrations(pool: &PgPool) -> PersistenceResult<()> {
        // Run the initial schema migration
        sqlx::query(include_str!("../../../migrations/001_initial_pg.sql"))
            .execute(pool)
            .await
            .map_err(|e| PersistenceError::Migration(e.to_string()))?;
        Ok(())
    }

    /// Close the database connection pool.
    pub async fn close(&self) {
        match self {
            #[cfg(feature = "sqlite")]
            DatabasePool::Sqlite(pool) => pool.close().await,
            #[cfg(feature = "postgres")]
            DatabasePool::Postgres(pool) => pool.close().await,
            #[cfg(not(any(feature = "sqlite", feature = "postgres")))]
            DatabasePool::None => {}
        }
    }

    /// Check if the database is healthy.
    pub async fn is_healthy(&self) -> bool {
        match self {
            #[cfg(feature = "sqlite")]
            DatabasePool::Sqlite(pool) => {
                sqlx::query("SELECT 1").execute(pool).await.is_ok()
            }
            #[cfg(feature = "postgres")]
            DatabasePool::Postgres(pool) => {
                sqlx::query("SELECT 1").execute(pool).await.is_ok()
            }
            #[cfg(not(any(feature = "sqlite", feature = "postgres")))]
            DatabasePool::None => false,
        }
    }
}

/// Stored phase transition record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPhaseTransition {
    pub id: i64,
    pub from_phase: String,
    pub to_phase: String,
    pub cycle_number: i64,
    pub transitioned_at: DateTime<Utc>,
    pub metrics_json: String,
    pub created_at: DateTime<Utc>,
}

/// Stored metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMetricsSnapshot {
    pub id: i64,
    pub cycle_number: i64,
    pub phase: String,
    pub active_connections: i64,
    pub total_connections: i64,
    pub messages_received: i64,
    pub messages_sent: i64,
    pub spectral_k: f64,
    pub mean_metabolic_trust: f64,
    pub active_wounds: i64,
    pub composting_entities: i64,
    pub liminal_entities: i64,
    pub entangled_pairs: i64,
    pub held_uncertainties: i64,
    pub snapshot_at: DateTime<Utc>,
}

/// Stored event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub id: i64,
    pub event_type: String,
    pub event_data: String,
    pub cycle_number: i64,
    pub phase: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PersistenceConfig::default();
        assert_eq!(config.database_url, "sqlite:./mycelix.db");
        assert_eq!(config.retention_days, 30);
        assert!(config.auto_migrate);
    }

    #[test]
    fn test_detect_backend() {
        assert_eq!(
            PersistenceConfig::detect_backend("sqlite:./test.db"),
            DatabaseBackend::Sqlite
        );
        assert_eq!(
            PersistenceConfig::detect_backend("postgres://localhost/test"),
            DatabaseBackend::Postgres
        );
        assert_eq!(
            PersistenceConfig::detect_backend("postgresql://localhost/test"),
            DatabaseBackend::Postgres
        );
    }

    #[test]
    fn test_config_validation() {
        let config = PersistenceConfig::default();
        // SQLite should always be valid when sqlite feature is enabled
        #[cfg(feature = "sqlite")]
        assert!(config.validate().is_ok());

        let empty_url = PersistenceConfig {
            database_url: "".to_string(),
            ..Default::default()
        };
        assert!(empty_url.validate().is_err());

        let zero_retention = PersistenceConfig {
            retention_days: 0,
            ..Default::default()
        };
        assert!(zero_retention.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = PersistenceConfig::sqlite("./test.db")
            .with_retention_days(60)
            .with_metrics_interval(120)
            .without_auto_migrate();

        assert_eq!(config.database_url, "sqlite:./test.db");
        assert_eq!(config.retention_days, 60);
        assert_eq!(config.metrics_snapshot_interval_secs, 120);
        assert!(!config.auto_migrate);
    }
}
