//! WebSocket RPC Server for Mycelix Living Protocol
//!
//! Provides a WebSocket server that handles JSON-RPC style requests from
//! TypeScript clients and broadcasts protocol events.
//!
//! # Protocol
//!
//! ## Requests (client -> server)
//! ```json
//! { "id": "1", "method": "getCycleState", "params": {} }
//! ```
//!
//! ## Responses (server -> client)
//! ```json
//! { "id": "1", "result": { "cycleNumber": 1, "currentPhase": "Shadow", ... } }
//! { "id": "1", "error": { "code": -32601, "message": "Method not found" } }
//! ```
//!
//! ## Events (server -> client, no id)
//! ```json
//! { "type": "PhaseTransitioned", "data": { "from": "Shadow", "to": "Composting" } }
//! ```

mod rpc;
mod server;

pub use rpc::{
    RpcError, RpcRequest, RpcResponse, RPC_ERROR_INTERNAL, RPC_ERROR_INVALID_PARAMS,
    RPC_ERROR_METHOD_NOT_FOUND,
};
pub use server::{ServerConfig, WebSocketServer};
