//! Benchmarks for the WebSocket RPC server components.
//!
//! Tests RPC request parsing, response serialization, and concurrent request handling.
//!
//! Run with: `cargo bench --bench ws_server`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

// We'll create inline versions of the RPC types to avoid circular dependencies
// In production, these would come from ws_server crate

/// JSON-RPC error codes
const RPC_ERROR_METHOD_NOT_FOUND: i32 = -32601;
const RPC_ERROR_INVALID_PARAMS: i32 = -32602;
const RPC_ERROR_INTERNAL: i32 = -32603;

/// Incoming RPC request from client.
#[derive(Debug, Clone, serde::Deserialize)]
struct RpcRequest {
    id: String,
    method: String,
    #[serde(default)]
    params: Value,
}

/// RPC error response.
#[derive(Debug, Clone, serde::Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl RpcError {
    fn method_not_found(method: &str) -> Self {
        Self {
            code: RPC_ERROR_METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
        }
    }

    fn invalid_params(detail: &str) -> Self {
        Self {
            code: RPC_ERROR_INVALID_PARAMS,
            message: format!("Invalid params: {}", detail),
        }
    }

    fn internal(detail: &str) -> Self {
        Self {
            code: RPC_ERROR_INTERNAL,
            message: format!("Internal error: {}", detail),
        }
    }
}

/// RPC response to client.
#[derive(Debug, Clone, serde::Serialize)]
struct RpcResponse {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

impl RpcResponse {
    fn success(id: String, result: impl serde::Serialize) -> Self {
        Self {
            id,
            result: Some(serde_json::to_value(result).unwrap_or(Value::Null)),
            error: None,
        }
    }

    fn error(id: String, error: RpcError) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// Cycle state response (mock).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CycleStateResponse {
    cycle_number: u64,
    current_phase: String,
    phase_started: String,
    cycle_started: String,
    phase_day: u32,
}

impl Default for CycleStateResponse {
    fn default() -> Self {
        Self {
            cycle_number: 42,
            current_phase: "Shadow".to_string(),
            phase_started: "2024-01-15T00:00:00Z".to_string(),
            cycle_started: "2024-01-12T00:00:00Z".to_string(),
            phase_day: 3,
        }
    }
}

/// Phase transition response (mock).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PhaseTransitionResponse {
    from: String,
    to: String,
    cycle_number: u64,
    transitioned_at: String,
}

// Sample request payloads
fn sample_requests() -> Vec<(&'static str, String)> {
    vec![
        ("getCycleState_minimal", r#"{"id":"1","method":"getCycleState"}"#.to_string()),
        ("getCycleState_with_params", r#"{"id":"2","method":"getCycleState","params":{}}"#.to_string()),
        ("getCurrentPhase", r#"{"id":"3","method":"getCurrentPhase","params":{}}"#.to_string()),
        ("getCycleNumber", r#"{"id":"4","method":"getCycleNumber","params":{}}"#.to_string()),
        ("getPhaseMetrics", r#"{"id":"5","method":"getPhaseMetrics","params":{"phase":"Shadow"}}"#.to_string()),
        ("getTransitionHistory", r#"{"id":"6","method":"getTransitionHistory","params":{}}"#.to_string()),
        ("isOperationPermitted", r#"{"id":"7","method":"isOperationPermitted","params":{"operation":"create_proposal"}}"#.to_string()),
        ("unknown_method", r#"{"id":"8","method":"unknownMethod","params":{}}"#.to_string()),
    ]
}

// Sample responses for serialization benchmarks
fn sample_responses() -> Vec<(&'static str, RpcResponse)> {
    vec![
        ("success_cycle_state", RpcResponse::success("1".to_string(), CycleStateResponse::default())),
        ("success_phase", RpcResponse::success("2".to_string(), "Shadow")),
        ("success_number", RpcResponse::success("3".to_string(), 42u64)),
        ("success_bool", RpcResponse::success("4".to_string(), true)),
        ("success_history", RpcResponse::success("5".to_string(), vec![
            PhaseTransitionResponse {
                from: "Kenosis".to_string(),
                to: "Shadow".to_string(),
                cycle_number: 41,
                transitioned_at: "2024-01-12T00:00:00Z".to_string(),
            },
            PhaseTransitionResponse {
                from: "Shadow".to_string(),
                to: "Composting".to_string(),
                cycle_number: 42,
                transitioned_at: "2024-01-15T00:00:00Z".to_string(),
            },
        ])),
        ("error_method_not_found", RpcResponse::error("6".to_string(), RpcError::method_not_found("foo"))),
        ("error_invalid_params", RpcResponse::error("7".to_string(), RpcError::invalid_params("missing phase"))),
    ]
}

/// Benchmark RPC request parsing.
fn bench_request_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("rpc_request_parsing");
    group.throughput(Throughput::Elements(1));

    for (name, json) in sample_requests() {
        group.bench_with_input(BenchmarkId::new("parse", name), &json, |b, json| {
            b.iter(|| {
                let request: RpcRequest = serde_json::from_str(black_box(json)).unwrap();
                black_box(request)
            })
        });
    }

    group.finish();
}

/// Benchmark RPC request parsing with validation.
fn bench_request_parsing_with_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("rpc_request_validation");

    for (name, json) in sample_requests() {
        group.bench_with_input(BenchmarkId::new("validate", name), &json, |b, json| {
            b.iter(|| {
                let result: Result<RpcRequest, _> = serde_json::from_str(black_box(json));
                match result {
                    Ok(request) => {
                        // Validate method
                        let valid = matches!(
                            request.method.as_str(),
                            "getCycleState"
                                | "getCurrentPhase"
                                | "getCycleNumber"
                                | "getPhaseMetrics"
                                | "getTransitionHistory"
                                | "isOperationPermitted"
                        );
                        black_box((request, valid))
                    }
                    Err(e) => black_box((
                        RpcRequest {
                            id: String::new(),
                            method: String::new(),
                            params: Value::Null,
                        },
                        false,
                    )),
                }
            })
        });
    }

    group.finish();
}

/// Benchmark RPC response serialization.
fn bench_response_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("rpc_response_serialization");
    group.throughput(Throughput::Elements(1));

    for (name, response) in sample_responses() {
        group.bench_with_input(BenchmarkId::new("serialize", name), &response, |b, response| {
            b.iter(|| {
                let json = serde_json::to_string(black_box(response)).unwrap();
                black_box(json)
            })
        });
    }

    group.finish();
}

/// Benchmark response creation and serialization.
fn bench_response_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("rpc_response_creation");

    group.bench_function("success_simple", |b| {
        b.iter(|| {
            let response = RpcResponse::success(black_box("1".to_string()), black_box(42u64));
            let json = serde_json::to_string(&response).unwrap();
            black_box(json)
        })
    });

    group.bench_function("success_complex", |b| {
        b.iter(|| {
            let state = CycleStateResponse::default();
            let response = RpcResponse::success(black_box("1".to_string()), state);
            let json = serde_json::to_string(&response).unwrap();
            black_box(json)
        })
    });

    group.bench_function("error_response", |b| {
        b.iter(|| {
            let response = RpcResponse::error(
                black_box("1".to_string()),
                RpcError::method_not_found(black_box("unknown")),
            );
            let json = serde_json::to_string(&response).unwrap();
            black_box(json)
        })
    });

    group.finish();
}

/// Benchmark batch request parsing.
fn bench_batch_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_parsing");

    for batch_size in [1, 10, 50, 100] {
        let requests: Vec<String> = (0..batch_size)
            .map(|i| {
                format!(
                    r#"{{"id":"{}","method":"getCycleState","params":{{}}}}"#,
                    i
                )
            })
            .collect();

        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            &requests,
            |b, requests| {
                b.iter(|| {
                    let parsed: Vec<RpcRequest> = requests
                        .iter()
                        .map(|r| serde_json::from_str(r).unwrap())
                        .collect();
                    black_box(parsed)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark simulated request handling (parse + process + serialize).
fn bench_request_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_handling");
    group.throughput(Throughput::Elements(1));

    // Simulated handler function
    fn handle_request(request: &RpcRequest) -> RpcResponse {
        match request.method.as_str() {
            "getCycleState" => RpcResponse::success(request.id.clone(), CycleStateResponse::default()),
            "getCurrentPhase" => RpcResponse::success(request.id.clone(), "Shadow"),
            "getCycleNumber" => RpcResponse::success(request.id.clone(), 42u64),
            "getPhaseMetrics" => {
                let phase = request.params.get("phase").and_then(|v| v.as_str());
                match phase {
                    Some(_) => RpcResponse::success(request.id.clone(), json!({
                        "operations": 100,
                        "averageLatency": 5.2,
                    })),
                    None => RpcResponse::error(
                        request.id.clone(),
                        RpcError::invalid_params("missing phase"),
                    ),
                }
            }
            _ => RpcResponse::error(request.id.clone(), RpcError::method_not_found(&request.method)),
        }
    }

    for (name, json) in sample_requests() {
        group.bench_with_input(BenchmarkId::new("handle", name), &json, |b, json| {
            b.iter(|| {
                // Parse request
                let request: RpcRequest = serde_json::from_str(black_box(json)).unwrap();
                // Handle request
                let response = handle_request(&request);
                // Serialize response
                let response_json = serde_json::to_string(&response).unwrap();
                black_box(response_json)
            })
        });
    }

    group.finish();
}

/// Benchmark concurrent request handling simulation.
fn bench_concurrent_handling(c: &mut Criterion) {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    let mut group = c.benchmark_group("concurrent_handling");
    group.measurement_time(Duration::from_secs(10));

    // Shared counter to simulate engine state
    let counter = Arc::new(AtomicU64::new(0));

    fn handle_request_concurrent(request: &RpcRequest, counter: &AtomicU64) -> RpcResponse {
        // Simulate some processing
        counter.fetch_add(1, Ordering::Relaxed);

        match request.method.as_str() {
            "getCycleState" => {
                let state = CycleStateResponse {
                    cycle_number: counter.load(Ordering::Relaxed),
                    ..Default::default()
                };
                RpcResponse::success(request.id.clone(), state)
            }
            "getCurrentPhase" => RpcResponse::success(request.id.clone(), "Shadow"),
            _ => RpcResponse::error(request.id.clone(), RpcError::method_not_found(&request.method)),
        }
    }

    for num_threads in [1, 2, 4, 8] {
        let requests: Vec<String> = (0..1000)
            .map(|i| format!(r#"{{"id":"{}","method":"getCycleState","params":{{}}}}"#, i))
            .collect();

        group.throughput(Throughput::Elements(1000));
        group.bench_with_input(
            BenchmarkId::new("threads", num_threads),
            &requests,
            |b, requests| {
                let counter = Arc::clone(&counter);
                b.iter(|| {
                    let chunk_size = requests.len() / num_threads;
                    let handles: Vec<_> = (0..num_threads)
                        .map(|t| {
                            let reqs = requests[t * chunk_size..(t + 1) * chunk_size].to_vec();
                            let counter = Arc::clone(&counter);
                            thread::spawn(move || {
                                let mut results = Vec::with_capacity(reqs.len());
                                for json in &reqs {
                                    let request: RpcRequest = serde_json::from_str(json).unwrap();
                                    let response = handle_request_concurrent(&request, &counter);
                                    let json = serde_json::to_string(&response).unwrap();
                                    results.push(json);
                                }
                                results
                            })
                        })
                        .collect();

                    let results: Vec<Vec<String>> = handles.into_iter().map(|h| h.join().unwrap()).collect();
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark JSON value extraction from params.
fn bench_param_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("param_extraction");

    let params_simple = json!({"phase": "Shadow"});
    let params_complex = json!({
        "phase": "Shadow",
        "includeMetrics": true,
        "filters": {
            "minOperations": 10,
            "maxLatency": 100.0,
            "tags": ["important", "recent"]
        },
        "pagination": {
            "offset": 0,
            "limit": 50
        }
    });

    group.bench_function("extract_simple", |b| {
        b.iter(|| {
            let phase = black_box(&params_simple)
                .get("phase")
                .and_then(|v| v.as_str());
            black_box(phase)
        })
    });

    group.bench_function("extract_complex", |b| {
        b.iter(|| {
            let params = black_box(&params_complex);
            let phase = params.get("phase").and_then(|v| v.as_str());
            let include = params.get("includeMetrics").and_then(|v| v.as_bool());
            let min_ops = params
                .get("filters")
                .and_then(|f| f.get("minOperations"))
                .and_then(|v| v.as_i64());
            let limit = params
                .get("pagination")
                .and_then(|p| p.get("limit"))
                .and_then(|v| v.as_i64());
            black_box((phase, include, min_ops, limit))
        })
    });

    group.finish();
}

/// Benchmark string operations (ID generation, method matching).
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    // Method matching
    let methods = [
        "getCycleState",
        "getCurrentPhase",
        "getCycleNumber",
        "getPhaseMetrics",
        "getTransitionHistory",
        "isOperationPermitted",
        "unknownMethod",
    ];

    group.bench_function("method_match", |b| {
        b.iter(|| {
            for method in &methods {
                let is_valid = matches!(
                    *method,
                    "getCycleState"
                        | "getCurrentPhase"
                        | "getCycleNumber"
                        | "getPhaseMetrics"
                        | "getTransitionHistory"
                        | "isOperationPermitted"
                );
                black_box(is_valid);
            }
        })
    });

    // ID generation
    use std::sync::atomic::{AtomicU64, Ordering};
    let counter = AtomicU64::new(0);

    group.bench_function("id_generation_atomic", |b| {
        b.iter(|| {
            let id = counter.fetch_add(1, Ordering::Relaxed);
            let id_str = format!("req-{}", id);
            black_box(id_str)
        })
    });

    group.bench_function("id_generation_uuid", |b| {
        b.iter(|| {
            let id = uuid::Uuid::new_v4().to_string();
            black_box(id)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_request_parsing,
    bench_request_parsing_with_validation,
    bench_response_serialization,
    bench_response_creation,
    bench_batch_parsing,
    bench_request_handling,
    bench_concurrent_handling,
    bench_param_extraction,
    bench_string_operations,
);

criterion_main!(benches);
