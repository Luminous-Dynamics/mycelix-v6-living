//! Middleware architecture for RPC request/response interception.
//!
//! Middlewares can intercept, transform, or reject requests and responses
//! flowing through the cycle engine's RPC layer.
//!
//! # Example
//!
//! ```rust,ignore
//! use cycle_engine::middleware::{Middleware, MiddlewareChain, RpcRequest, RpcResponse};
//!
//! struct MyMiddleware;
//!
//! impl Middleware for MyMiddleware {
//!     fn name(&self) -> &str { "my-middleware" }
//!
//!     fn handle_request(&self, req: RpcRequest, next: MiddlewareNext) -> MiddlewareResult {
//!         println!("Request: {}", req.method);
//!         next.run(req)
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

// =============================================================================
// RPC Types
// =============================================================================

/// An RPC request to the cycle engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    /// Unique request ID
    pub id: String,
    /// JSON-RPC method name
    pub method: String,
    /// Request parameters
    pub params: serde_json::Value,
    /// Request metadata/headers
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Timestamp when the request was received
    pub timestamp: DateTime<Utc>,
}

impl RpcRequest {
    /// Create a new RPC request.
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            method: method.into(),
            params,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Add metadata to the request.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get a metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
}

/// An RPC response from the cycle engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// Request ID this responds to
    pub id: String,
    /// Response result (if successful)
    pub result: Option<serde_json::Value>,
    /// Error (if failed)
    pub error: Option<RpcError>,
    /// Response metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Timestamp when the response was generated
    pub timestamp: DateTime<Utc>,
}

impl RpcResponse {
    /// Create a successful response.
    pub fn success(id: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            result: Some(result),
            error: None,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Create an error response.
    pub fn error(id: impl Into<String>, error: RpcError) -> Self {
        Self {
            id: id.into(),
            result: None,
            error: Some(error),
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Add metadata to the response.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if the response is successful.
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Check if the response is an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// An RPC error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    /// Create a new RPC error.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Add data to the error.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    // Standard error codes
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const UNAUTHORIZED: i32 = -32001;
    pub const RATE_LIMITED: i32 = -32002;
    pub const PHASE_RESTRICTED: i32 = -32003;
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

// =============================================================================
// Middleware Trait
// =============================================================================

/// Result of middleware processing.
pub type MiddlewareResult = Result<RpcResponse, MiddlewareError>;

/// Errors that can occur in middleware processing.
#[derive(Debug, thiserror::Error)]
pub enum MiddlewareError {
    #[error("Request rejected: {0}")]
    Rejected(String),

    #[error("Middleware error: {0}")]
    Internal(String),

    #[error("Chain interrupted")]
    ChainInterrupted,
}

/// The main middleware trait.
///
/// Middlewares can intercept requests before they reach the handler
/// and responses before they're returned to the client.
pub trait Middleware: Send + Sync {
    /// Unique name of the middleware.
    fn name(&self) -> &str;

    /// Process an incoming request.
    ///
    /// Call `next.run(req)` to continue to the next middleware.
    /// Return a response directly to short-circuit the chain.
    fn handle_request(
        &self,
        req: RpcRequest,
        next: MiddlewareNext<'_>,
    ) -> MiddlewareResult;

    /// Process an outgoing response (optional).
    ///
    /// Default implementation passes through unchanged.
    fn handle_response(&self, resp: RpcResponse) -> RpcResponse {
        resp
    }

    /// Check if this middleware should process the given method.
    ///
    /// Return false to skip this middleware for certain methods.
    fn should_process(&self, _method: &str) -> bool {
        true
    }

    /// Get middleware-specific metrics.
    fn metrics(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

/// Continuation for the middleware chain.
pub struct MiddlewareNext<'a> {
    chain: &'a MiddlewareChain,
    index: usize,
    handler: &'a dyn Fn(RpcRequest) -> MiddlewareResult,
}

impl<'a> MiddlewareNext<'a> {
    /// Continue to the next middleware in the chain.
    pub fn run(self, req: RpcRequest) -> MiddlewareResult {
        self.chain.process_at(req, self.index + 1, self.handler)
    }
}

// =============================================================================
// Middleware Chain
// =============================================================================

/// A chain of middlewares that process requests in order.
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Create a new empty middleware chain.
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the end of the chain.
    pub fn add(&mut self, middleware: impl Middleware + 'static) {
        self.middlewares.push(Arc::new(middleware));
    }

    /// Add a middleware to the end of the chain (Arc version).
    pub fn add_arc(&mut self, middleware: Arc<dyn Middleware>) {
        self.middlewares.push(middleware);
    }

    /// Insert a middleware at a specific position.
    pub fn insert(&mut self, index: usize, middleware: impl Middleware + 'static) {
        self.middlewares.insert(index, Arc::new(middleware));
    }

    /// Remove a middleware by name.
    pub fn remove(&mut self, name: &str) -> bool {
        let len_before = self.middlewares.len();
        self.middlewares.retain(|m| m.name() != name);
        self.middlewares.len() < len_before
    }

    /// Get the number of middlewares in the chain.
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// Get the names of all middlewares in order.
    pub fn middleware_names(&self) -> Vec<&str> {
        self.middlewares.iter().map(|m| m.name()).collect()
    }

    /// Process a request through the chain.
    pub fn process(
        &self,
        req: RpcRequest,
        handler: &dyn Fn(RpcRequest) -> MiddlewareResult,
    ) -> MiddlewareResult {
        self.process_at(req, 0, handler)
    }

    /// Process a request starting at a specific index.
    fn process_at(
        &self,
        req: RpcRequest,
        index: usize,
        handler: &dyn Fn(RpcRequest) -> MiddlewareResult,
    ) -> MiddlewareResult {
        // Find the next middleware that should process this request
        let next_index = (index..self.middlewares.len())
            .find(|&i| self.middlewares[i].should_process(&req.method));

        match next_index {
            Some(i) => {
                let middleware = &self.middlewares[i];
                let next = MiddlewareNext {
                    chain: self,
                    index: i,
                    handler,
                };

                let result = middleware.handle_request(req, next);

                // Apply response processing
                match result {
                    Ok(resp) => Ok(middleware.handle_response(resp)),
                    Err(e) => Err(e),
                }
            }
            None => {
                // No more middlewares, call the handler
                handler(req)
            }
        }
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for MiddlewareChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MiddlewareChain")
            .field("middlewares", &self.middleware_names())
            .finish()
    }
}

// =============================================================================
// Built-in Middlewares
// =============================================================================

/// Middleware that logs all requests and responses.
pub struct LoggingMiddleware {
    /// Log level for requests
    log_params: bool,
    /// Log level for responses
    log_results: bool,
}

impl LoggingMiddleware {
    /// Create a new logging middleware.
    pub fn new() -> Self {
        Self {
            log_params: false,
            log_results: false,
        }
    }

    /// Enable logging of request parameters.
    pub fn with_params(mut self) -> Self {
        self.log_params = true;
        self
    }

    /// Enable logging of response results.
    pub fn with_results(mut self) -> Self {
        self.log_results = true;
        self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for LoggingMiddleware {
    fn name(&self) -> &str {
        "logging"
    }

    fn handle_request(
        &self,
        req: RpcRequest,
        next: MiddlewareNext<'_>,
    ) -> MiddlewareResult {
        if self.log_params {
            info!(
                request_id = %req.id,
                method = %req.method,
                params = %req.params,
                "RPC request"
            );
        } else {
            info!(
                request_id = %req.id,
                method = %req.method,
                "RPC request"
            );
        }

        let start = Instant::now();
        let result = next.run(req.clone());
        let duration = start.elapsed();

        match &result {
            Ok(resp) => {
                if resp.is_success() {
                    if self.log_results {
                        info!(
                            request_id = %req.id,
                            method = %req.method,
                            duration_ms = %duration.as_millis(),
                            result = ?resp.result,
                            "RPC response (success)"
                        );
                    } else {
                        info!(
                            request_id = %req.id,
                            method = %req.method,
                            duration_ms = %duration.as_millis(),
                            "RPC response (success)"
                        );
                    }
                } else {
                    warn!(
                        request_id = %req.id,
                        method = %req.method,
                        duration_ms = %duration.as_millis(),
                        error = ?resp.error,
                        "RPC response (error)"
                    );
                }
            }
            Err(e) => {
                error!(
                    request_id = %req.id,
                    method = %req.method,
                    duration_ms = %duration.as_millis(),
                    error = %e,
                    "RPC middleware error"
                );
            }
        }

        result
    }
}

/// Middleware that collects metrics about RPC calls.
pub struct MetricsMiddleware {
    /// Track request counts by method
    request_counts: std::sync::Mutex<HashMap<String, u64>>,
    /// Track error counts by method
    error_counts: std::sync::Mutex<HashMap<String, u64>>,
    /// Track request durations by method (in milliseconds)
    request_durations: std::sync::Mutex<HashMap<String, Vec<u64>>>,
    /// Maximum number of durations to keep per method
    max_durations: usize,
}

impl MetricsMiddleware {
    /// Create a new metrics middleware.
    pub fn new() -> Self {
        Self {
            request_counts: std::sync::Mutex::new(HashMap::new()),
            error_counts: std::sync::Mutex::new(HashMap::new()),
            request_durations: std::sync::Mutex::new(HashMap::new()),
            max_durations: 1000,
        }
    }

    /// Get request count for a method.
    pub fn request_count(&self, method: &str) -> u64 {
        self.request_counts
            .lock()
            .unwrap()
            .get(method)
            .copied()
            .unwrap_or(0)
    }

    /// Get error count for a method.
    pub fn error_count(&self, method: &str) -> u64 {
        self.error_counts
            .lock()
            .unwrap()
            .get(method)
            .copied()
            .unwrap_or(0)
    }

    /// Get average duration for a method in milliseconds.
    pub fn average_duration_ms(&self, method: &str) -> Option<f64> {
        let durations = self.request_durations.lock().unwrap();
        durations.get(method).and_then(|d| {
            if d.is_empty() {
                None
            } else {
                Some(d.iter().sum::<u64>() as f64 / d.len() as f64)
            }
        })
    }

    /// Get p95 duration for a method in milliseconds.
    pub fn p95_duration_ms(&self, method: &str) -> Option<u64> {
        let durations = self.request_durations.lock().unwrap();
        durations.get(method).and_then(|d| {
            if d.is_empty() {
                None
            } else {
                let mut sorted = d.clone();
                sorted.sort_unstable();
                let idx = (sorted.len() as f64 * 0.95) as usize;
                sorted.get(idx.min(sorted.len() - 1)).copied()
            }
        })
    }

    /// Get total request count across all methods.
    pub fn total_request_count(&self) -> u64 {
        self.request_counts.lock().unwrap().values().sum()
    }

    /// Get total error count across all methods.
    pub fn total_error_count(&self) -> u64 {
        self.error_counts.lock().unwrap().values().sum()
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        self.request_counts.lock().unwrap().clear();
        self.error_counts.lock().unwrap().clear();
        self.request_durations.lock().unwrap().clear();
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for MetricsMiddleware {
    fn name(&self) -> &str {
        "metrics"
    }

    fn handle_request(
        &self,
        req: RpcRequest,
        next: MiddlewareNext<'_>,
    ) -> MiddlewareResult {
        let method = req.method.clone();

        // Increment request count
        {
            let mut counts = self.request_counts.lock().unwrap();
            *counts.entry(method.clone()).or_insert(0) += 1;
        }

        let start = Instant::now();
        let result = next.run(req);
        let duration = start.elapsed();

        // Record duration
        {
            let mut durations = self.request_durations.lock().unwrap();
            let method_durations = durations.entry(method.clone()).or_insert_with(Vec::new);
            method_durations.push(duration.as_millis() as u64);

            // Keep bounded
            if method_durations.len() > self.max_durations {
                method_durations.remove(0);
            }
        }

        // Track errors
        match &result {
            Ok(resp) if resp.is_error() => {
                let mut errors = self.error_counts.lock().unwrap();
                *errors.entry(method).or_insert(0) += 1;
            }
            Err(_) => {
                let mut errors = self.error_counts.lock().unwrap();
                *errors.entry(method).or_insert(0) += 1;
            }
            _ => {}
        }

        result
    }

    fn metrics(&self) -> serde_json::Value {
        let request_counts = self.request_counts.lock().unwrap().clone();
        let error_counts = self.error_counts.lock().unwrap().clone();

        let mut method_metrics = serde_json::Map::new();
        for (method, count) in &request_counts {
            method_metrics.insert(
                method.clone(),
                serde_json::json!({
                    "requests": count,
                    "errors": error_counts.get(method).unwrap_or(&0),
                    "avg_duration_ms": self.average_duration_ms(method),
                    "p95_duration_ms": self.p95_duration_ms(method),
                }),
            );
        }

        serde_json::json!({
            "total_requests": self.total_request_count(),
            "total_errors": self.total_error_count(),
            "methods": method_metrics,
        })
    }
}

/// Middleware that enforces rate limiting.
pub struct RateLimitMiddleware {
    /// Maximum requests per window per client
    max_requests: u64,
    /// Window duration
    window: Duration,
    /// Request counts per client
    client_counts: std::sync::Mutex<HashMap<String, (u64, Instant)>>,
    /// Header name for client identification
    client_id_header: String,
}

impl RateLimitMiddleware {
    /// Create a new rate limit middleware.
    ///
    /// # Arguments
    ///
    /// * `max_requests` - Maximum requests allowed per window
    /// * `window` - Duration of the rate limit window
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            client_counts: std::sync::Mutex::new(HashMap::new()),
            client_id_header: "x-client-id".to_string(),
        }
    }

    /// Set the header name used for client identification.
    pub fn with_client_id_header(mut self, header: impl Into<String>) -> Self {
        self.client_id_header = header.into();
        self
    }

    /// Get the client ID from a request.
    fn get_client_id(&self, req: &RpcRequest) -> String {
        req.metadata
            .get(&self.client_id_header)
            .cloned()
            .unwrap_or_else(|| "anonymous".to_string())
    }

    /// Check if a client is rate limited.
    fn is_rate_limited(&self, client_id: &str) -> bool {
        let mut counts = self.client_counts.lock().unwrap();
        let now = Instant::now();

        match counts.get_mut(client_id) {
            Some((count, window_start)) => {
                if now.duration_since(*window_start) >= self.window {
                    // Window expired, reset
                    *count = 1;
                    *window_start = now;
                    false
                } else if *count >= self.max_requests {
                    true
                } else {
                    *count += 1;
                    false
                }
            }
            None => {
                counts.insert(client_id.to_string(), (1, now));
                false
            }
        }
    }

    /// Get remaining requests for a client.
    pub fn remaining_requests(&self, client_id: &str) -> u64 {
        let counts = self.client_counts.lock().unwrap();
        match counts.get(client_id) {
            Some((count, window_start)) => {
                if Instant::now().duration_since(*window_start) >= self.window {
                    self.max_requests
                } else {
                    self.max_requests.saturating_sub(*count)
                }
            }
            None => self.max_requests,
        }
    }
}

impl Middleware for RateLimitMiddleware {
    fn name(&self) -> &str {
        "rate-limit"
    }

    fn handle_request(
        &self,
        req: RpcRequest,
        next: MiddlewareNext<'_>,
    ) -> MiddlewareResult {
        let client_id = self.get_client_id(&req);

        if self.is_rate_limited(&client_id) {
            warn!(
                client_id = %client_id,
                method = %req.method,
                "Rate limit exceeded"
            );

            return Ok(RpcResponse::error(
                req.id,
                RpcError::new(RpcError::RATE_LIMITED, "Rate limit exceeded")
                    .with_data(serde_json::json!({
                        "retry_after_secs": self.window.as_secs(),
                    })),
            ));
        }

        let remaining = self.remaining_requests(&client_id);
        let resp = next.run(req)?;

        Ok(resp.with_metadata("x-ratelimit-remaining", remaining.to_string()))
    }
}

/// Middleware that validates request parameters.
pub struct ValidationMiddleware {
    /// Schema validators by method name
    validators: HashMap<String, serde_json::Value>,
}

impl ValidationMiddleware {
    /// Create a new validation middleware.
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
        }
    }

    /// Register a JSON Schema validator for a method.
    pub fn register_schema(
        &mut self,
        method: impl Into<String>,
        schema: serde_json::Value,
    ) {
        self.validators.insert(method.into(), schema);
    }

    /// Simple validation (checks required fields exist).
    fn validate(&self, method: &str, params: &serde_json::Value) -> Result<(), String> {
        if let Some(schema) = self.validators.get(method) {
            // Simple required field validation
            if let Some(required) = schema.get("required") {
                if let Some(required_array) = required.as_array() {
                    for field in required_array {
                        if let Some(field_name) = field.as_str() {
                            if params.get(field_name).is_none() {
                                return Err(format!("Missing required field: {}", field_name));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for ValidationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for ValidationMiddleware {
    fn name(&self) -> &str {
        "validation"
    }

    fn handle_request(
        &self,
        req: RpcRequest,
        next: MiddlewareNext<'_>,
    ) -> MiddlewareResult {
        if let Err(e) = self.validate(&req.method, &req.params) {
            debug!(
                method = %req.method,
                error = %e,
                "Validation failed"
            );

            return Ok(RpcResponse::error(
                req.id,
                RpcError::new(RpcError::INVALID_PARAMS, e),
            ));
        }

        next.run(req)
    }

    fn should_process(&self, method: &str) -> bool {
        self.validators.contains_key(method)
    }
}

/// Middleware that adds tracing context to requests.
pub struct TracingMiddleware {
    /// Header name for trace ID propagation
    trace_id_header: String,
}

impl TracingMiddleware {
    /// Create a new tracing middleware.
    pub fn new() -> Self {
        Self {
            trace_id_header: "x-trace-id".to_string(),
        }
    }

    /// Set the header name for trace ID.
    pub fn with_trace_id_header(mut self, header: impl Into<String>) -> Self {
        self.trace_id_header = header.into();
        self
    }
}

impl Default for TracingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for TracingMiddleware {
    fn name(&self) -> &str {
        "tracing"
    }

    fn handle_request(
        &self,
        mut req: RpcRequest,
        next: MiddlewareNext<'_>,
    ) -> MiddlewareResult {
        // Generate or propagate trace ID
        let trace_id = req
            .metadata
            .get(&self.trace_id_header)
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        req.metadata
            .insert(self.trace_id_header.clone(), trace_id.clone());

        let span = tracing::info_span!(
            "rpc_request",
            trace_id = %trace_id,
            method = %req.method,
            request_id = %req.id,
        );
        let _enter = span.enter();

        let resp = next.run(req)?;
        Ok(resp.with_metadata(&self.trace_id_header, trace_id))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handler(req: RpcRequest) -> MiddlewareResult {
        Ok(RpcResponse::success(
            req.id,
            serde_json::json!({"echo": req.method}),
        ))
    }

    #[test]
    fn test_middleware_chain_empty() {
        let chain = MiddlewareChain::new();
        let req = RpcRequest::new("test", serde_json::json!({}));

        let resp = chain.process(req, &test_handler).unwrap();
        assert!(resp.is_success());
    }

    #[test]
    fn test_logging_middleware() {
        let mut chain = MiddlewareChain::new();
        chain.add(LoggingMiddleware::new());

        let req = RpcRequest::new("test.method", serde_json::json!({"key": "value"}));
        let resp = chain.process(req, &test_handler).unwrap();

        assert!(resp.is_success());
    }

    #[test]
    fn test_metrics_middleware() {
        let metrics = Arc::new(MetricsMiddleware::new());
        let mut chain = MiddlewareChain::new();
        chain.add_arc(metrics.clone());

        // Make some requests
        for _ in 0..5 {
            let req = RpcRequest::new("test.method", serde_json::json!({}));
            chain.process(req, &test_handler).unwrap();
        }

        assert_eq!(metrics.request_count("test.method"), 5);
        assert_eq!(metrics.total_request_count(), 5);
        assert!(metrics.average_duration_ms("test.method").is_some());
    }

    #[test]
    fn test_rate_limit_middleware() {
        let mut chain = MiddlewareChain::new();
        chain.add(RateLimitMiddleware::new(2, Duration::from_secs(60)));

        let make_request = || {
            RpcRequest::new("test", serde_json::json!({}))
                .with_metadata("x-client-id", "test-client")
        };

        // First two requests should succeed
        let resp1 = chain.process(make_request(), &test_handler).unwrap();
        assert!(resp1.is_success());

        let resp2 = chain.process(make_request(), &test_handler).unwrap();
        assert!(resp2.is_success());

        // Third request should be rate limited
        let resp3 = chain.process(make_request(), &test_handler).unwrap();
        assert!(resp3.is_error());
        assert_eq!(resp3.error.unwrap().code, RpcError::RATE_LIMITED);
    }

    #[test]
    fn test_validation_middleware() {
        let mut validation = ValidationMiddleware::new();
        validation.register_schema(
            "user.create",
            serde_json::json!({
                "required": ["name", "email"]
            }),
        );

        let mut chain = MiddlewareChain::new();
        chain.add(validation);

        // Valid request
        let valid_req = RpcRequest::new(
            "user.create",
            serde_json::json!({"name": "Test", "email": "test@example.com"}),
        );
        let resp = chain.process(valid_req, &test_handler).unwrap();
        assert!(resp.is_success());

        // Invalid request (missing email)
        let invalid_req = RpcRequest::new("user.create", serde_json::json!({"name": "Test"}));
        let resp = chain.process(invalid_req, &test_handler).unwrap();
        assert!(resp.is_error());
        assert_eq!(resp.error.unwrap().code, RpcError::INVALID_PARAMS);

        // Unvalidated method passes through
        let other_req = RpcRequest::new("other.method", serde_json::json!({}));
        let resp = chain.process(other_req, &test_handler).unwrap();
        assert!(resp.is_success());
    }

    #[test]
    fn test_middleware_chain_ordering() {
        struct OrderTracker {
            name: &'static str,
            order: Arc<std::sync::Mutex<Vec<&'static str>>>,
        }

        impl Middleware for OrderTracker {
            fn name(&self) -> &str {
                self.name
            }

            fn handle_request(
                &self,
                req: RpcRequest,
                next: MiddlewareNext<'_>,
            ) -> MiddlewareResult {
                self.order.lock().unwrap().push(self.name);
                next.run(req)
            }
        }

        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let mut chain = MiddlewareChain::new();
        chain.add(OrderTracker {
            name: "first",
            order: order.clone(),
        });
        chain.add(OrderTracker {
            name: "second",
            order: order.clone(),
        });
        chain.add(OrderTracker {
            name: "third",
            order: order.clone(),
        });

        let req = RpcRequest::new("test", serde_json::json!({}));
        chain.process(req, &test_handler).unwrap();

        let recorded_order = order.lock().unwrap();
        assert_eq!(*recorded_order, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_rpc_request_metadata() {
        let req = RpcRequest::new("test", serde_json::json!({}))
            .with_metadata("x-custom", "value")
            .with_metadata("x-another", "another-value");

        assert_eq!(req.get_metadata("x-custom"), Some("value"));
        assert_eq!(req.get_metadata("x-another"), Some("another-value"));
        assert_eq!(req.get_metadata("x-missing"), None);
    }

    #[test]
    fn test_rpc_response() {
        let success = RpcResponse::success("1", serde_json::json!({"result": "ok"}));
        assert!(success.is_success());
        assert!(!success.is_error());

        let error = RpcResponse::error(
            "2",
            RpcError::new(RpcError::INTERNAL_ERROR, "Something went wrong"),
        );
        assert!(!error.is_success());
        assert!(error.is_error());
    }
}
