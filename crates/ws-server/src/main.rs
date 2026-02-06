//! Mycelix WebSocket RPC Server
//!
//! A standalone server for the Living Protocol that provides:
//! - WebSocket RPC for TypeScript/JavaScript clients
//! - Real-time event broadcasting
//! - Cycle engine management
//! - OpenTelemetry tracing support
//! - Security features (rate limiting, authentication)
//! - Optional REST API
//! - Optional GraphQL API with subscriptions
//! - Optional Server-Sent Events (SSE) endpoint
//! - Optional Webhook delivery
//!
//! # Usage
//!
//! ```bash
//! # Start with defaults (localhost:8888)
//! cargo run -p ws-server
//!
//! # Custom host and port
//! cargo run -p ws-server -- --host 0.0.0.0 --port 9000
//!
//! # With debug logging
//! cargo run -p ws-server -- --log-level debug
//!
//! # With JSON structured logging
//! cargo run -p ws-server -- --json-logs
//!
//! # With OTLP export (requires 'otlp' feature)
//! cargo run -p ws-server --features otlp -- --otlp-endpoint http://localhost:4317
//!
//! # With authentication
//! cargo run -p ws-server -- --require-auth --api-keys "key1,key2,key3"
//!
//! # With rate limiting
//! cargo run -p ws-server -- --rate-limit 100 --max-connections 5000
//!
//! # With REST API enabled
//! cargo run -p ws-server -- --enable-rest --rest-port 8890
//!
//! # With GraphQL API enabled (requires 'graphql' feature)
//! cargo run -p ws-server --features graphql -- --enable-graphql --graphql-port 8891
//!
//! # With SSE endpoint enabled (requires 'sse' feature)
//! cargo run -p ws-server --features sse -- --enable-sse --sse-port 8892
//!
//! # With webhook delivery (requires 'webhooks' feature)
//! cargo run -p ws-server --features webhooks -- --webhook-url https://example.com/webhook --webhook-secret mysecret --webhook-events PhaseTransitioned,CycleStarted
//!
//! # With database persistence
//! cargo run -p ws-server --features sqlite -- --database-url sqlite:./mycelix.db
//!
//! # With PostgreSQL (requires postgres feature)
//! cargo run -p ws-server --features postgres -- --database-url postgres://user:pass@localhost/mycelix
//! ```

use std::net::SocketAddr;

use clap::Parser;
use tracing::info;

use ws_server::{
    telemetry::{self, TelemetryConfig},
    AdminConfig, AdminServer, AuthConfig, RateLimitConfig, RestConfig, RestServer,
    ServerConfig, ServerConfigResponse, WebSocketServer,
};

#[cfg(any(feature = "sqlite", feature = "postgres"))]
use ws_server::ServerMetrics;

#[cfg(any(feature = "sqlite", feature = "postgres"))]
use ws_server::{PersistenceConfig, create_repository};

/// Mycelix Living Protocol WebSocket RPC Server
#[derive(Parser, Debug)]
#[command(name = "mycelix-ws-server")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Host address to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on for WebSocket connections
    #[arg(short, long, default_value_t = 8888)]
    port: u16,

    /// Port to listen on for health/metrics HTTP
    #[arg(long, default_value_t = 8889)]
    health_port: u16,

    /// Disable health/metrics HTTP server
    #[arg(long)]
    no_health: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Enable JSON structured logging output
    #[arg(long)]
    json_logs: bool,

    /// OpenTelemetry OTLP endpoint for trace export (e.g., http://localhost:4317)
    #[arg(long)]
    otlp_endpoint: Option<String>,

    /// Enable simulated time (accelerated cycle for testing)
    #[arg(long)]
    simulated_time: bool,

    /// Time acceleration factor (only with --simulated-time)
    #[arg(long, default_value_t = 1.0)]
    time_acceleration: f64,

    // === Security Options ===
    /// Maximum total WebSocket connections (global limit)
    #[arg(long, default_value_t = 10000)]
    max_connections: u32,

    /// Maximum connections per IP address
    #[arg(long, default_value_t = 10)]
    max_connections_per_ip: u32,

    /// Maximum requests per second per IP (rate limit)
    #[arg(long, default_value_t = 100)]
    rate_limit: u32,

    /// Rate limit burst size (token bucket capacity)
    #[arg(long, default_value_t = 200)]
    rate_limit_burst: u32,

    /// Require authentication for all connections
    #[arg(long)]
    require_auth: bool,

    /// Comma-separated list of valid API keys (used with --require-auth)
    #[arg(long, value_delimiter = ',')]
    api_keys: Vec<String>,

    // === REST API Options ===
    /// Enable REST API endpoints alongside WebSocket
    #[arg(long)]
    enable_rest: bool,

    /// Port for REST API server (only with --enable-rest)
    #[arg(long, default_value_t = 8890)]
    rest_port: u16,

    /// CORS allowed origins for REST API (comma-separated, empty = allow all)
    #[arg(long, value_delimiter = ',')]
    cors_origins: Vec<String>,

    // === GraphQL Options (requires 'graphql' feature) ===
    /// Enable GraphQL API with subscriptions
    #[arg(long)]
    enable_graphql: bool,

    /// Port for GraphQL server (only with --enable-graphql)
    #[arg(long, default_value_t = 8891)]
    graphql_port: u16,

    /// Disable GraphQL Playground
    #[arg(long)]
    no_graphql_playground: bool,

    // === SSE Options (requires 'sse' feature) ===
    /// Enable Server-Sent Events endpoint at /api/v1/events
    #[arg(long)]
    enable_sse: bool,

    /// Port for SSE server (only with --enable-sse)
    #[arg(long, default_value_t = 8892)]
    sse_port: u16,

    // === Webhook Options (requires 'webhooks' feature) ===
    /// Webhook URL to deliver events to
    #[arg(long)]
    webhook_url: Option<String>,

    /// Secret key for webhook HMAC-SHA256 signing
    #[arg(long)]
    webhook_secret: Option<String>,

    /// Comma-separated list of event types to send to webhook (empty = all events)
    #[arg(long, value_delimiter = ',')]
    webhook_events: Vec<String>,

    // === Admin Panel Options ===
    /// Enable web-based admin panel
    #[arg(long)]
    enable_admin: bool,

    /// Port for admin panel server (only with --enable-admin)
    #[arg(long, default_value_t = 8891)]
    admin_port: u16,

    /// Password for admin panel (basic auth, username is "admin")
    #[arg(long)]
    admin_password: Option<String>,

    // === Persistence Options ===
    /// Database URL for persistence (sqlite:./path.db or postgres://user:pass@host/db)
    #[arg(long, default_value = "sqlite:./mycelix.db")]
    database_url: String,

    /// Number of days to retain historical metrics and events
    #[arg(long, default_value_t = 30)]
    metrics_retention_days: u32,

    /// Interval in seconds for saving metrics snapshots to database
    #[arg(long, default_value_t = 60)]
    metrics_snapshot_interval: u64,

    /// Disable database persistence
    #[arg(long)]
    no_persistence: bool,

    /// Disable automatic database migrations
    #[arg(long)]
    no_auto_migrate: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize telemetry (logging + optional OpenTelemetry)
    let telemetry_config = TelemetryConfig {
        log_level: args.log_level.clone(),
        json_logs: args.json_logs,
        otlp_endpoint: args.otlp_endpoint.clone(),
        service_name: "mycelix-ws-server".to_string(),
    };

    let tracer_provider = telemetry::init_telemetry(&telemetry_config)?;

    // Warn if OTLP endpoint is specified but feature is not enabled
    #[cfg(not(feature = "otlp"))]
    if args.otlp_endpoint.is_some() {
        tracing::warn!(
            "OTLP endpoint specified but 'otlp' feature is not enabled. \
             Rebuild with --features otlp to enable trace export."
        );
    }

    #[cfg(feature = "otlp")]
    if args.otlp_endpoint.is_some() {
        info!(
            endpoint = %args.otlp_endpoint.as_ref().unwrap(),
            "OTLP trace export enabled"
        );
    }

    // Parse bind address
    let bind_addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

    // Parse health address
    let health_addr = if args.no_health {
        None
    } else {
        Some(
            format!("{}:{}", args.host, args.health_port)
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid health address: {}", e))?,
        )
    };

    info!("Starting Mycelix WebSocket RPC Server");
    info!(address = %bind_addr, "Binding WebSocket server");

    if let Some(addr) = &health_addr {
        info!(address = %addr, "Health/metrics endpoint enabled");
    }

    if args.simulated_time {
        info!(
            acceleration = args.time_acceleration,
            "Simulated time enabled"
        );
    }

    if args.json_logs {
        info!("JSON structured logging enabled");
    }

    // Create rate limit configuration
    let rate_limit_config = RateLimitConfig {
        requests_per_second: args.rate_limit,
        burst_size: args.rate_limit_burst,
        max_connections_per_ip: args.max_connections_per_ip,
        max_total_connections: args.max_connections,
        ..Default::default()
    };

    info!(
        max_connections = args.max_connections,
        max_per_ip = args.max_connections_per_ip,
        rate_limit = args.rate_limit,
        burst_size = args.rate_limit_burst,
        "Rate limiting configured"
    );

    // Create authentication configuration
    let auth_config = if args.require_auth {
        if args.api_keys.is_empty() {
            return Err(anyhow::anyhow!(
                "--require-auth specified but no --api-keys provided. \
                 Please provide at least one API key."
            ));
        }
        info!(
            key_count = args.api_keys.len(),
            "Authentication enabled with API keys"
        );
        AuthConfig::with_api_keys(args.api_keys.clone())
    } else if !args.api_keys.is_empty() {
        info!(
            key_count = args.api_keys.len(),
            "API keys configured (optional authentication)"
        );
        let mut config = AuthConfig::default();
        for key in &args.api_keys {
            config.add_api_key(key);
        }
        config
    } else {
        info!("Authentication disabled (anonymous access allowed)");
        AuthConfig::default()
    };

    // Create server config
    let config = ServerConfig {
        bind_addr,
        health_addr,
        broadcast_capacity: 1024,
        rate_limit: rate_limit_config,
        auth: auth_config,
    };

    // Initialize persistence layer if enabled
    #[cfg(any(feature = "sqlite", feature = "postgres"))]
    let repository = if !args.no_persistence {
        let persistence_config = PersistenceConfig {
            database_url: args.database_url.clone(),
            backend: None, // Auto-detect from URL
            retention_days: args.metrics_retention_days,
            auto_migrate: !args.no_auto_migrate,
            metrics_snapshot_interval_secs: args.metrics_snapshot_interval,
            ..Default::default()
        };

        match create_repository(&persistence_config).await {
            Ok(repo) => {
                info!(
                    database_url = %args.database_url,
                    retention_days = args.metrics_retention_days,
                    "Database persistence enabled"
                );
                Some(repo)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to initialize database persistence, continuing without it"
                );
                None
            }
        }
    } else {
        info!("Database persistence disabled");
        None
    };

    // Create and run server
    let server = if args.simulated_time {
        use cycle_engine::CycleEngineBuilder;
        let engine = CycleEngineBuilder::new()
            .with_simulated_time(args.time_acceleration)
            .build();
        WebSocketServer::with_engine(config, engine)
    } else {
        WebSocketServer::new(config)
    };

    // Start persistence background tasks if enabled
    #[cfg(any(feature = "sqlite", feature = "postgres"))]
    if let Some(repo) = repository.clone() {
        let engine_for_persistence = server.engine();
        let metrics_for_persistence = server.metrics();
        let retention_days = args.metrics_retention_days;
        let snapshot_interval = args.metrics_snapshot_interval;

        // Spawn metrics snapshot task
        tokio::spawn(async move {
            use tokio::time::{interval, Duration};
            use std::sync::atomic::Ordering;

            let mut tick = interval(Duration::from_secs(snapshot_interval));
            let mut cleanup_counter = 0u64;

            loop {
                tick.tick().await;

                // Get current state
                let engine = engine_for_persistence.read().await;
                let cycle_number = engine.cycle_number();
                let phase = engine.current_phase();
                let phase_metrics = engine.phase_metrics(phase);
                drop(engine);

                // Get server metrics
                let server_metrics = ServerMetrics {
                    active_connections: metrics_for_persistence.active_connections.load(Ordering::Relaxed),
                    total_connections: metrics_for_persistence.total_connections.load(Ordering::Relaxed),
                    messages_received: metrics_for_persistence.messages_received.load(Ordering::Relaxed),
                    messages_sent: metrics_for_persistence.messages_sent.load(Ordering::Relaxed),
                    uptime_seconds: 0, // Not easily accessible here
                };

                // Save metrics snapshot
                if let Err(e) = repo.save_metrics(cycle_number, phase, &server_metrics, &phase_metrics).await {
                    tracing::warn!(error = %e, "Failed to save metrics snapshot");
                }

                // Run cleanup every hour (60 snapshots at 60s interval)
                cleanup_counter += 1;
                if cleanup_counter >= 60 {
                    cleanup_counter = 0;
                    if let Err(e) = repo.cleanup_old_data(retention_days).await {
                        tracing::warn!(error = %e, "Failed to cleanup old data");
                    }
                }
            }
        });
    }

    info!(
        ws_url = format!("ws://{}:{}", args.host, args.port),
        "Server ready, waiting for connections"
    );

    // Start admin panel server if enabled
    if args.enable_admin {
        let admin_addr: SocketAddr = format!("{}:{}", args.host, args.admin_port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid admin address: {}", e))?;

        let admin_config = AdminConfig {
            bind_addr: admin_addr,
            password: args.admin_password.clone(),
            test_mode: args.simulated_time,
        };

        let server_config_response = ServerConfigResponse {
            bind_addr: bind_addr.to_string(),
            health_addr: health_addr.map(|a| a.to_string()),
            max_connections: args.max_connections,
            max_connections_per_ip: args.max_connections_per_ip,
            rate_limit: args.rate_limit,
            rate_limit_burst: args.rate_limit_burst,
            auth_required: args.require_auth,
            test_mode: args.simulated_time,
        };

        let admin_server = AdminServer::new(
            admin_config,
            server.engine(),
            server.metrics(),
            server_config_response,
        );

        info!(
            admin_url = format!("http://{}:{}", args.host, args.admin_port),
            auth = args.admin_password.is_some(),
            "Admin panel enabled"
        );

        // Run admin server in background
        tokio::spawn(async move {
            if let Err(e) = admin_server.run().await {
                tracing::error!(error = %e, "Admin panel server error");
            }
        });
    }

    // Start REST API server if enabled
    if args.enable_rest {
        let rest_addr: SocketAddr = format!("{}:{}", args.host, args.rest_port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid REST address: {}", e))?;

        let rest_config = RestConfig {
            bind_addr: rest_addr,
            cors_origins: if args.cors_origins.is_empty() {
                None
            } else {
                Some(args.cors_origins.clone())
            },
        };

        let rest_server = RestServer::new(rest_config, server.engine(), server.metrics());

        info!(
            rest_url = format!("http://{}:{}", args.host, args.rest_port),
            "REST API enabled"
        );

        // Run REST server in background
        tokio::spawn(async move {
            if let Err(e) = rest_server.run().await {
                tracing::error!(error = %e, "REST API server error");
            }
        });
    }

    // Start GraphQL API server if enabled
    #[cfg(feature = "graphql")]
    if args.enable_graphql {
        use ws_server::{run_graphql_server, GraphQLConfig};

        let graphql_config = GraphQLConfig {
            port: args.graphql_port,
            host: args.host.clone(),
            playground: !args.no_graphql_playground,
            introspection: true,
        };

        let engine = server.engine();
        let event_tx = server.event_sender();

        info!(
            graphql_url = format!("http://{}:{}/graphql", args.host, args.graphql_port),
            playground = !args.no_graphql_playground,
            "GraphQL API enabled"
        );

        tokio::spawn(async move {
            if let Err(e) = run_graphql_server(graphql_config, engine, event_tx).await {
                tracing::error!(error = %e, "GraphQL server error");
            }
        });
    }

    #[cfg(not(feature = "graphql"))]
    if args.enable_graphql {
        tracing::warn!(
            "--enable-graphql specified but 'graphql' feature is not enabled. \
             Rebuild with --features graphql to enable GraphQL support."
        );
    }

    // Start SSE server if enabled
    #[cfg(feature = "sse")]
    if args.enable_sse {
        use ws_server::{run_sse_server, SseConfig};

        let sse_config = SseConfig {
            port: args.sse_port,
            host: args.host.clone(),
            keep_alive_seconds: 30,
        };

        let event_tx = server.event_sender();

        info!(
            sse_url = format!("http://{}:{}/api/v1/events", args.host, args.sse_port),
            "SSE endpoint enabled"
        );

        tokio::spawn(async move {
            if let Err(e) = run_sse_server(sse_config, event_tx).await {
                tracing::error!(error = %e, "SSE server error");
            }
        });
    }

    #[cfg(not(feature = "sse"))]
    if args.enable_sse {
        tracing::warn!(
            "--enable-sse specified but 'sse' feature is not enabled. \
             Rebuild with --features sse to enable SSE support."
        );
    }

    // Start webhook dispatcher if configured
    #[cfg(feature = "webhooks")]
    if let Some(webhook_url) = args.webhook_url {
        use std::sync::Arc as WebhookArc;
        use ws_server::{WebhookConfig, WebhookManager};

        let webhook_secret = args.webhook_secret.unwrap_or_else(|| {
            tracing::warn!("No --webhook-secret provided, using empty secret. This is insecure!");
            String::new()
        });

        let events: std::collections::HashSet<String> = if args.webhook_events.is_empty() {
            std::collections::HashSet::new() // All events
        } else {
            args.webhook_events.into_iter().collect()
        };

        let webhook_config = WebhookConfig::new(webhook_url.clone(), webhook_secret)
            .with_events(events.clone());

        let manager = WebhookArc::new(WebhookManager::new());
        manager.register(webhook_config).await;

        let event_rx = server.event_sender().subscribe();

        info!(
            webhook_url = %webhook_url,
            event_filter = ?events,
            "Webhook delivery enabled"
        );

        let manager_clone = WebhookArc::clone(&manager);
        tokio::spawn(async move {
            manager_clone.start_dispatcher(event_rx).await;
        });
    }

    #[cfg(not(feature = "webhooks"))]
    if args.webhook_url.is_some() {
        tracing::warn!(
            "--webhook-url specified but 'webhooks' feature is not enabled. \
             Rebuild with --features webhooks to enable webhook support."
        );
    }

    let result = server.run().await;

    // Gracefully shutdown telemetry
    telemetry::shutdown_telemetry(tracer_provider);

    result
}
