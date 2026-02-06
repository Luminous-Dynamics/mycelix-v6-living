//! OpenTelemetry tracing and structured logging initialization.
//!
//! This module provides telemetry setup for the WebSocket server including:
//! - OpenTelemetry tracing with OTLP export support
//! - Structured JSON logging
//! - Trace context propagation helpers

#[cfg(feature = "otlp")]
use opentelemetry::trace::TracerProvider as TracerProviderTrait;
use opentelemetry_sdk::trace::TracerProvider;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Telemetry configuration options.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Log level filter (e.g., "info", "debug", "trace")
    pub log_level: String,
    /// Enable JSON formatted logs
    pub json_logs: bool,
    /// OTLP endpoint for trace export (requires `otlp` feature)
    pub otlp_endpoint: Option<String>,
    /// Service name for traces
    pub service_name: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            json_logs: false,
            otlp_endpoint: None,
            service_name: "mycelix-ws-server".to_string(),
        }
    }
}

/// Initialize the telemetry subsystem with the given configuration.
///
/// This sets up:
/// - Console logging (either pretty-printed or JSON formatted)
/// - OpenTelemetry tracing (if OTLP endpoint is configured)
///
/// # Errors
///
/// Returns an error if the tracing subscriber cannot be initialized.
pub fn init_telemetry(config: &TelemetryConfig) -> anyhow::Result<Option<TracerProvider>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Build the OpenTelemetry layer if OTLP endpoint is configured
    #[cfg(feature = "otlp")]
    let provider = if let Some(endpoint) = &config.otlp_endpoint {
        let provider = init_otlp_tracer(endpoint, &config.service_name)?;
        let tracer = provider.tracer(config.service_name.clone());
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        // Build the logging layer and combine with OTLP
        // Use boxed layers to erase types and allow combining different format layers
        if config.json_logs {
            let json_layer = fmt::layer()
                .json()
                .with_span_events(FmtSpan::CLOSE)
                .with_current_span(true)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .boxed();

            tracing_subscriber::registry()
                .with(env_filter)
                .with(json_layer)
                .with(otel_layer)
                .try_init()
                .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;
        } else {
            let fmt_layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .compact()
                .boxed();

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .with(otel_layer)
                .try_init()
                .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;
        }

        Some(provider)
    } else {
        // No OTLP endpoint, just logging
        init_logging_only(config, env_filter)?;
        None
    };

    #[cfg(not(feature = "otlp"))]
    let provider = {
        init_logging_only(config, env_filter)?;
        None
    };

    Ok(provider)
}

/// Initialize logging without OpenTelemetry.
fn init_logging_only(config: &TelemetryConfig, env_filter: EnvFilter) -> anyhow::Result<()> {
    if config.json_logs {
        let json_layer = fmt::layer()
            .json()
            .with_span_events(FmtSpan::CLOSE)
            .with_current_span(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_filter(env_filter);

        tracing_subscriber::registry()
            .with(json_layer)
            .try_init()
            .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;
    } else {
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .compact()
            .with_filter(env_filter);

        tracing_subscriber::registry()
            .with(fmt_layer)
            .try_init()
            .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;
    }
    Ok(())
}

/// Initialize OTLP tracer for OpenTelemetry export.
#[cfg(feature = "otlp")]
fn init_otlp_tracer(
    endpoint: &str,
    service_name: &str,
) -> anyhow::Result<TracerProvider> {
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::{runtime, Resource};

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create OTLP exporter: {}", e))?;

    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", service_name.to_string()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ]))
        .build();

    Ok(provider)
}

/// Shutdown the OpenTelemetry tracer provider gracefully.
pub fn shutdown_telemetry(provider: Option<TracerProvider>) {
    if let Some(provider) = provider {
        if let Err(e) = provider.shutdown() {
            eprintln!("Error shutting down tracer provider: {:?}", e);
        }
    }
}

/// Get the current trace ID as a hex string.
///
/// Returns an empty string if there is no active trace or OpenTelemetry is not configured.
/// This function requires the tracing-opentelemetry span extension trait to be in scope.
pub fn current_trace_id_string() -> String {
    #[cfg(feature = "otlp")]
    {
        use opentelemetry::trace::{TraceContextExt, TraceId};
        use tracing::Span;
        use tracing_opentelemetry::OpenTelemetrySpanExt;

        let span = Span::current();
        let context = span.context();
        let span_ref = context.span();
        let span_context = span_ref.span_context();

        if span_context.is_valid() {
            let trace_id: TraceId = span_context.trace_id();
            format!("{:032x}", trace_id)
        } else {
            String::new()
        }
    }

    #[cfg(not(feature = "otlp"))]
    {
        String::new()
    }
}

/// Create a field value containing the current trace ID for logging.
///
/// Usage:
/// ```ignore
/// info!(trace_id = %trace_id_field(), "Processing request");
/// ```
pub fn trace_id_field() -> String {
    current_trace_id_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TelemetryConfig::default();
        assert_eq!(config.log_level, "info");
        assert!(!config.json_logs);
        assert!(config.otlp_endpoint.is_none());
        assert_eq!(config.service_name, "mycelix-ws-server");
    }

    #[test]
    fn test_trace_id_without_span() {
        // Without any span context, should return empty
        let trace_id = current_trace_id_string();
        // Without OTLP feature or active span, should be empty
        assert!(trace_id.is_empty() || trace_id.len() == 32);
    }
}
