//! Mycelix WebSocket RPC Server
//!
//! A standalone server for the Living Protocol that provides:
//! - WebSocket RPC for TypeScript/JavaScript clients
//! - Real-time event broadcasting
//! - Cycle engine management
//! - OpenTelemetry tracing support
//! - Security features (rate limiting, authentication)
//! - Optional REST API
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
//! ```

use std::net::SocketAddr;

use clap::Parser;
use tracing::info;

use ws_server::{
    telemetry::{self, TelemetryConfig},
    AuthConfig, RateLimitConfig, RestConfig, RestServer, ServerConfig, WebSocketServer,
};

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
    } else {
        if !args.api_keys.is_empty() {
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
        }
    };

    // Create server config
    let config = ServerConfig {
        bind_addr,
        health_addr,
        broadcast_capacity: 1024,
        rate_limit: rate_limit_config,
        auth: auth_config,
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

    info!(
        ws_url = format!("ws://{}:{}", args.host, args.port),
        "Server ready, waiting for connections"
    );

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

    let result = server.run().await;

    // Gracefully shutdown telemetry
    telemetry::shutdown_telemetry(tracer_provider);

    result
}
