//! JSON-RPC types for WebSocket communication.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC error codes
pub const RPC_ERROR_METHOD_NOT_FOUND: i32 = -32601;
pub const RPC_ERROR_INVALID_PARAMS: i32 = -32602;
pub const RPC_ERROR_INTERNAL: i32 = -32603;

/// Incoming RPC request from client.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcRequest {
    /// Request ID for correlation
    pub id: String,
    /// Method name to invoke
    pub method: String,
    /// Optional parameters
    #[serde(default)]
    pub params: Value,
}

/// RPC error response.
#[derive(Debug, Clone, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcError {
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: RPC_ERROR_METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
        }
    }

    pub fn invalid_params(detail: &str) -> Self {
        Self {
            code: RPC_ERROR_INVALID_PARAMS,
            message: format!("Invalid params: {}", detail),
        }
    }

    pub fn internal(detail: &str) -> Self {
        Self {
            code: RPC_ERROR_INTERNAL,
            message: format!("Internal error: {}", detail),
        }
    }
}

/// RPC response to client.
#[derive(Debug, Clone, Serialize)]
pub struct RpcResponse {
    /// Request ID for correlation
    pub id: String,
    /// Result on success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error on failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn success(id: String, result: impl Serialize) -> Self {
        Self {
            id,
            result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
            error: None,
        }
    }

    pub fn error(id: String, error: RpcError) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_parsing() {
        let json = r#"{"id": "1", "method": "getCycleState"}"#;
        let request: RpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "1");
        assert_eq!(request.method, "getCycleState");
        assert!(request.params.is_null());
    }

    #[test]
    fn test_request_with_params() {
        let json = r#"{"id": "2", "method": "getPhaseMetrics", "params": {"phase": "Shadow"}}"#;
        let request: RpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "2");
        assert_eq!(request.params["phase"], "Shadow");
    }

    #[test]
    fn test_success_response() {
        let response = RpcResponse::success("1".to_string(), serde_json::json!({"cycleNumber": 1}));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"1\""));
        assert!(json.contains("\"cycleNumber\":1"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_error_response() {
        let response = RpcResponse::error("1".to_string(), RpcError::method_not_found("foo"));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"1\""));
        assert!(json.contains("-32601"));
        assert!(!json.contains("result"));
    }
}
