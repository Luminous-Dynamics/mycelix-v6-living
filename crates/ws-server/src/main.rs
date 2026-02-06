//! Mycelix WebSocket RPC Server
//!
//! A standalone server for the Living Protocol that provides:
//! - WebSocket RPC for TypeScript/JavaScript clients
//! - Real-time event broadcasting
//! - Cycle engine management
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
//! ```

use std::net::SocketAddr;

use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use ws_server::{ServerConfig, WebSocketServer};

/// Mycelix Living Protocol WebSocket RPC Server
#[derive(Parser, Debug)]
#[command(name = "mycelix-ws-server")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Host address to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8888)]
    port: u16,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Enable simulated time (accelerated cycle for testing)
    #[arg(long)]
    simulated_time: bool,

    /// Time acceleration factor (only with --simulated-time)
    #[arg(long, default_value_t = 1.0)]
    time_acceleration: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Set up logging
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => {
            eprintln!("Invalid log level '{}', using 'info'", args.log_level);
            Level::INFO
        }
    };

    FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();

    // Parse bind address
    let bind_addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

    info!("Starting Mycelix WebSocket RPC Server");
    info!("Binding to {}", bind_addr);

    if args.simulated_time {
        info!(
            "Simulated time enabled with {}x acceleration",
            args.time_acceleration
        );
    }

    // Create server config
    let config = ServerConfig {
        bind_addr,
        broadcast_capacity: 1024,
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

    info!("Server ready. Waiting for connections...");
    info!("Connect with: ws://{}:{}", args.host, args.port);

    server.run().await
}
