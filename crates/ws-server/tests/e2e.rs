//! End-to-end integration tests for the WebSocket RPC server.
//!
//! These tests verify the full client-server communication loop:
//! - Server startup and WebSocket handshake
//! - RPC request/response
//! - Event broadcasting
//!
//! # Running
//!
//! ```bash
//! cargo test -p ws-server --test e2e
//! ```

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use ws_server::{ServerConfig, WebSocketServer};

/// Helper to find an available port.
fn get_available_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// Helper to start a server and return its address.
async fn start_test_server() -> (SocketAddr, oneshot::Sender<()>) {
    let port = get_available_port();
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    let config = ServerConfig {
        bind_addr: addr,
        health_addr: None, // Disable health server in tests
        broadcast_capacity: 64,
    };

    // Create server with simulated time for faster tests
    let engine = cycle_engine::CycleEngineBuilder::new()
        .with_simulated_time(1000.0) // 1000x speed
        .build();
    let server = WebSocketServer::with_engine(config, engine);

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    // Spawn server in background
    tokio::spawn(async move {
        tokio::select! {
            result = server.run() => {
                if let Err(e) = result {
                    eprintln!("Server error: {}", e);
                }
            }
            _ = &mut shutdown_rx => {
                // Shutdown requested
            }
        }
    });

    // Wait for server to be ready
    for _ in 0..50 {
        if TcpStream::connect(&addr).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    (addr, shutdown_tx)
}

/// Helper to connect a WebSocket client.
async fn connect_client(addr: SocketAddr) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let url = format!("ws://{}", addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to WebSocket server");
    ws_stream
}

/// Helper to send an RPC request and get response.
async fn rpc_call(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    id: &str,
    method: &str,
    params: Option<Value>,
) -> Value {
    let request = if let Some(p) = params {
        json!({ "id": id, "method": method, "params": p })
    } else {
        json!({ "id": id, "method": method })
    };

    ws.send(Message::Text(request.to_string())).await.unwrap();

    // Wait for response with matching ID
    loop {
        let msg = timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("Timeout waiting for response")
            .expect("Stream ended")
            .expect("WebSocket error");

        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text).unwrap();
            if response.get("id").and_then(|v| v.as_str()) == Some(id) {
                return response;
            }
            // Otherwise it's an event, keep waiting
        }
    }
}

#[tokio::test]
async fn test_get_cycle_state() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    let response = rpc_call(&mut ws, "1", "getCycleState", None).await;

    assert!(response.get("result").is_some(), "Expected result in response");
    let result = response.get("result").unwrap();

    assert!(result.get("cycleNumber").is_some());
    assert!(result.get("currentPhase").is_some());
    assert!(result.get("phaseStarted").is_some());
    assert!(result.get("cycleStarted").is_some());
    assert!(result.get("phaseDay").is_some());

    // Verify initial state
    assert_eq!(result["cycleNumber"], 1);
    assert_eq!(result["currentPhase"], "Shadow");
}

#[tokio::test]
async fn test_get_current_phase() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    let response = rpc_call(&mut ws, "2", "getCurrentPhase", None).await;

    assert!(response.get("result").is_some());
    assert_eq!(response["result"], "Shadow");
}

#[tokio::test]
async fn test_get_cycle_number() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    let response = rpc_call(&mut ws, "3", "getCycleNumber", None).await;

    assert!(response.get("result").is_some());
    assert_eq!(response["result"], 1);
}

#[tokio::test]
async fn test_get_transition_history() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    let response = rpc_call(&mut ws, "4", "getTransitionHistory", None).await;

    assert!(response.get("result").is_some());
    // Initially empty history
    assert!(response["result"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_get_phase_metrics() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    let response = rpc_call(
        &mut ws,
        "5",
        "getPhaseMetrics",
        Some(json!({ "phase": "Shadow" })),
    )
    .await;

    assert!(response.get("result").is_some());
    let result = response.get("result").unwrap();

    // Verify metrics structure
    assert!(result.get("active_agents").is_some());
    assert!(result.get("spectral_k").is_some());
    assert!(result.get("mean_metabolic_trust").is_some());
}

#[tokio::test]
async fn test_get_phase_metrics_invalid_phase() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    let response = rpc_call(
        &mut ws,
        "6",
        "getPhaseMetrics",
        Some(json!({ "phase": "InvalidPhase" })),
    )
    .await;

    assert!(response.get("error").is_some());
    assert_eq!(response["error"]["code"], -32602); // Invalid params
}

#[tokio::test]
async fn test_is_operation_permitted() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    // In Shadow phase, voting should be permitted
    let response = rpc_call(
        &mut ws,
        "7",
        "isOperationPermitted",
        Some(json!({ "operation": "vote" })),
    )
    .await;

    assert!(response.get("result").is_some());
    assert_eq!(response["result"], true);
}

#[tokio::test]
async fn test_unknown_method() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    let response = rpc_call(&mut ws, "8", "unknownMethod", None).await;

    assert!(response.get("error").is_some());
    assert_eq!(response["error"]["code"], -32601); // Method not found
}

#[tokio::test]
async fn test_ping_pong() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    // Send ping
    ws.send(Message::Text(json!({"type": "ping"}).to_string()))
        .await
        .unwrap();

    // Expect pong
    let msg = timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("Timeout")
        .expect("Stream ended")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let response: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(response["type"], "pong");
    } else {
        panic!("Expected text message");
    }
}

#[tokio::test]
async fn test_multiple_concurrent_requests() {
    let (addr, _shutdown) = start_test_server().await;
    let mut ws = connect_client(addr).await;

    // Send multiple requests without waiting for responses
    let requests = vec![
        json!({ "id": "a", "method": "getCycleState" }),
        json!({ "id": "b", "method": "getCurrentPhase" }),
        json!({ "id": "c", "method": "getCycleNumber" }),
    ];

    for req in &requests {
        ws.send(Message::Text(req.to_string())).await.unwrap();
    }

    // Collect responses
    let mut responses = Vec::new();
    for _ in 0..3 {
        let msg = timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("Timeout")
            .expect("Stream ended")
            .expect("WebSocket error");

        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text).unwrap();
            if response.get("id").is_some() {
                responses.push(response);
            }
        }
    }

    assert_eq!(responses.len(), 3);

    // Verify all requests got responses
    let ids: Vec<&str> = responses
        .iter()
        .filter_map(|r| r.get("id").and_then(|v| v.as_str()))
        .collect();
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
}

/// Helper to start a server with health endpoint.
async fn start_test_server_with_health() -> (SocketAddr, SocketAddr, oneshot::Sender<()>) {
    let ws_port = get_available_port();
    let health_port = get_available_port();
    let ws_addr: SocketAddr = format!("127.0.0.1:{}", ws_port).parse().unwrap();
    let health_addr: SocketAddr = format!("127.0.0.1:{}", health_port).parse().unwrap();

    let config = ServerConfig {
        bind_addr: ws_addr,
        health_addr: Some(health_addr),
        broadcast_capacity: 64,
    };

    let engine = cycle_engine::CycleEngineBuilder::new()
        .with_simulated_time(1000.0)
        .build();
    let server = WebSocketServer::with_engine(config, engine);

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        tokio::select! {
            result = server.run() => {
                if let Err(e) = result {
                    eprintln!("Server error: {}", e);
                }
            }
            _ = &mut shutdown_rx => {}
        }
    });

    // Wait for both servers to be ready
    for _ in 0..50 {
        if TcpStream::connect(&ws_addr).await.is_ok()
            && TcpStream::connect(&health_addr).await.is_ok()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    (ws_addr, health_addr, shutdown_tx)
}

/// Helper to make an HTTP request and get response body.
async fn http_get(addr: SocketAddr, path: &str) -> (String, String) {
    let mut stream = TcpStream::connect(addr).await.expect("Failed to connect");
    let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, addr);
    stream.write_all(request.as_bytes()).await.expect("Failed to write");

    let mut response = String::new();
    stream.read_to_string(&mut response).await.expect("Failed to read");

    // Parse status and body
    let parts: Vec<&str> = response.splitn(2, "\r\n\r\n").collect();
    let headers = parts[0];
    let body = parts.get(1).unwrap_or(&"").to_string();
    let status = headers.lines().next().unwrap_or("").to_string();

    (status, body)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (_ws_addr, health_addr, _shutdown) = start_test_server_with_health().await;

    let (status, body) = http_get(health_addr, "/health").await;

    assert!(status.contains("200 OK"), "Expected 200 OK, got: {}", status);
    let response: Value = serde_json::from_str(&body).expect("Invalid JSON");
    assert_eq!(response["status"], "healthy");
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let (_ws_addr, health_addr, _shutdown) = start_test_server_with_health().await;

    let (status, body) = http_get(health_addr, "/metrics").await;

    assert!(status.contains("200 OK"), "Expected 200 OK, got: {}", status);
    let response: Value = serde_json::from_str(&body).expect("Invalid JSON");

    assert!(response.get("activeConnections").is_some());
    assert!(response.get("totalConnections").is_some());
    assert!(response.get("messagesReceived").is_some());
    assert!(response.get("messagesSent").is_some());
}

#[tokio::test]
async fn test_state_endpoint() {
    let (_ws_addr, health_addr, _shutdown) = start_test_server_with_health().await;

    let (status, body) = http_get(health_addr, "/state").await;

    assert!(status.contains("200 OK"), "Expected 200 OK, got: {}", status);
    let response: Value = serde_json::from_str(&body).expect("Invalid JSON");

    assert!(response.get("cycleNumber").is_some());
    assert!(response.get("currentPhase").is_some());
    assert!(response.get("phaseStarted").is_some());
    assert!(response.get("cycleStarted").is_some());
    assert!(response.get("phaseDay").is_some());
}
