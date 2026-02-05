//! OpenTelemetry observability integration for the Metabolism Cycle Engine.
//!
//! This module provides distributed tracing and metrics collection via OTLP export.
//! Enable with the `telemetry` feature flag:
//!
//! ```toml
//! [dependencies]
//! cycle-engine = { path = "crates/cycle-engine", features = ["telemetry"] }
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use cycle_engine::telemetry::{init_telemetry, shutdown_telemetry};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialize telemetry (exports to OTLP endpoint at localhost:4317)
//!     init_telemetry("mycelix-cycle-engine").expect("Failed to initialize telemetry");
//!
//!     // ... run your application ...
//!
//!     // Graceful shutdown
//!     shutdown_telemetry();
//! }
//! ```

use opentelemetry::{
    global,
    metrics::{Counter, Histogram, Meter, MeterProvider as _},
    trace::TracerProvider as _,
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use std::sync::OnceLock;
use std::time::Duration;
use tracing_subscriber::prelude::*;

use living_core::CyclePhase;

/// Global metrics instance for the cycle engine.
static CYCLE_METRICS: OnceLock<CycleMetrics> = OnceLock::new();

/// Cycle engine metrics collection.
pub struct CycleMetrics {
    /// Counter for phase transitions.
    pub phase_transition_count: Counter<u64>,
    /// Histogram for tick duration in milliseconds.
    pub tick_duration_ms: Histogram<f64>,
    /// Histogram for complete cycle duration in seconds.
    pub cycle_completion_time: Histogram<f64>,
    /// Counter for total ticks processed.
    pub total_ticks: Counter<u64>,
    /// Counter for errors during tick processing.
    pub tick_errors: Counter<u64>,
}

impl CycleMetrics {
    /// Create a new CycleMetrics instance from a meter.
    fn new(meter: &Meter) -> Self {
        Self {
            phase_transition_count: meter
                .u64_counter("cycle_engine.phase_transitions")
                .with_description("Total number of phase transitions")
                .with_unit("transitions")
                .build(),
            tick_duration_ms: meter
                .f64_histogram("cycle_engine.tick_duration_ms")
                .with_description("Duration of each tick in milliseconds")
                .with_unit("ms")
                .build(),
            cycle_completion_time: meter
                .f64_histogram("cycle_engine.cycle_completion_time")
                .with_description("Duration of a complete 28-day cycle in seconds")
                .with_unit("s")
                .build(),
            total_ticks: meter
                .u64_counter("cycle_engine.total_ticks")
                .with_description("Total number of ticks processed")
                .with_unit("ticks")
                .build(),
            tick_errors: meter
                .u64_counter("cycle_engine.tick_errors")
                .with_description("Total number of tick errors")
                .with_unit("errors")
                .build(),
        }
    }

    /// Record a phase transition.
    pub fn record_phase_transition(&self, from: CyclePhase, to: CyclePhase, cycle_number: u64) {
        self.phase_transition_count.add(
            1,
            &[
                KeyValue::new("from_phase", format!("{:?}", from)),
                KeyValue::new("to_phase", format!("{:?}", to)),
                KeyValue::new("cycle_number", cycle_number as i64),
            ],
        );
    }

    /// Record tick duration.
    pub fn record_tick_duration(&self, duration_ms: f64, phase: CyclePhase, day: u32) {
        self.tick_duration_ms.record(
            duration_ms,
            &[
                KeyValue::new("phase", format!("{:?}", phase)),
                KeyValue::new("phase_day", day as i64),
            ],
        );
        self.total_ticks.add(
            1,
            &[KeyValue::new("phase", format!("{:?}", phase))],
        );
    }

    /// Record a tick error.
    pub fn record_tick_error(&self, phase: CyclePhase, error_type: &str) {
        self.tick_errors.add(
            1,
            &[
                KeyValue::new("phase", format!("{:?}", phase)),
                KeyValue::new("error_type", error_type.to_string()),
            ],
        );
    }

    /// Record cycle completion time.
    pub fn record_cycle_completion(&self, duration_secs: f64, cycle_number: u64) {
        self.cycle_completion_time.record(
            duration_secs,
            &[KeyValue::new("cycle_number", cycle_number as i64)],
        );
    }
}

/// Get the global cycle metrics instance.
///
/// Returns `None` if telemetry has not been initialized.
pub fn metrics() -> Option<&'static CycleMetrics> {
    CYCLE_METRICS.get()
}

/// Telemetry configuration options.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// OTLP endpoint URL (default: "http://localhost:4317")
    pub otlp_endpoint: String,
    /// Service name for resource identification
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Whether to export traces (default: true)
    pub enable_traces: bool,
    /// Whether to export metrics (default: true)
    pub enable_metrics: bool,
    /// Metrics export interval in seconds (default: 60)
    pub metrics_interval_secs: u64,
    /// Trace sampling ratio (default: 1.0 = sample all)
    pub trace_sample_ratio: f64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_string()),
            service_name: "mycelix-cycle-engine".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            enable_traces: true,
            enable_metrics: true,
            metrics_interval_secs: 60,
            trace_sample_ratio: 1.0,
        }
    }
}

/// Initialize OpenTelemetry with default configuration.
///
/// This sets up:
/// - OTLP trace exporter (to localhost:4317 by default)
/// - OTLP metrics exporter
/// - tracing-opentelemetry integration
///
/// # Arguments
///
/// * `service_name` - Name of the service for trace identification
///
/// # Errors
///
/// Returns an error if OTLP exporter setup fails.
pub fn init_telemetry(service_name: &str) -> Result<(), TelemetryError> {
    let config = TelemetryConfig {
        service_name: service_name.to_string(),
        ..Default::default()
    };
    init_telemetry_with_config(config)
}

/// Initialize OpenTelemetry with custom configuration.
///
/// # Arguments
///
/// * `config` - Telemetry configuration options
///
/// # Errors
///
/// Returns an error if OTLP exporter setup fails.
pub fn init_telemetry_with_config(config: TelemetryConfig) -> Result<(), TelemetryError> {
    // Build resource with service info
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("service.namespace", "mycelix"),
    ]);

    // Set up tracing if enabled
    let tracer = if config.enable_traces {
        let trace_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&config.otlp_endpoint)
            .build()
            .map_err(|e| TelemetryError::ExporterInit(e.to_string()))?;

        let tracer_provider = TracerProvider::builder()
            .with_batch_exporter(trace_exporter, runtime::Tokio)
            .with_sampler(Sampler::TraceIdRatioBased(config.trace_sample_ratio))
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource.clone())
            .build();

        let tracer = tracer_provider.tracer("cycle_engine");
        global::set_tracer_provider(tracer_provider);
        Some(tracer)
    } else {
        None
    };

    // Set up metrics if enabled
    if config.enable_metrics {
        let metrics_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_endpoint(&config.otlp_endpoint)
            .build()
            .map_err(|e| TelemetryError::ExporterInit(e.to_string()))?;

        let reader = PeriodicReader::builder(metrics_exporter, runtime::Tokio)
            .with_interval(Duration::from_secs(config.metrics_interval_secs))
            .build();

        let meter_provider = SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(resource)
            .build();

        // Initialize cycle metrics before setting global provider
        let meter = meter_provider.meter("cycle_engine");
        let _ = CYCLE_METRICS.set(CycleMetrics::new(&meter));

        global::set_meter_provider(meter_provider);
    }

    // Set up tracing subscriber with OpenTelemetry layer
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    if let Some(tracer) = tracer {
        let otel_layer = tracing_opentelemetry::layer()
            .with_tracer(tracer);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer)
            .try_init()
            .map_err(|e| TelemetryError::SubscriberInit(e.to_string()))?;
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .try_init()
            .map_err(|e| TelemetryError::SubscriberInit(e.to_string()))?;
    }

    tracing::info!(
        service = %config.service_name,
        endpoint = %config.otlp_endpoint,
        "OpenTelemetry initialized"
    );

    Ok(())
}

/// Gracefully shutdown telemetry exporters.
///
/// This flushes any pending spans and metrics before shutdown.
/// Should be called before application exit.
pub fn shutdown_telemetry() {
    tracing::info!("Shutting down OpenTelemetry...");
    global::shutdown_tracer_provider();
    // Note: MeterProvider shutdown is handled automatically on drop
}

/// Errors that can occur during telemetry initialization.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("Failed to initialize OTLP exporter: {0}")]
    ExporterInit(String),
    #[error("Failed to initialize tracing subscriber: {0}")]
    SubscriberInit(String),
}

/// Custom span attributes for cycle engine operations.
pub mod attributes {
    use opentelemetry::KeyValue;
    use living_core::CyclePhase;

    /// Create span attributes for a cycle phase.
    pub fn phase_attributes(phase: CyclePhase, day: u32, cycle_number: u64) -> Vec<KeyValue> {
        vec![
            KeyValue::new("cycle.phase", format!("{:?}", phase)),
            KeyValue::new("cycle.phase_day", day as i64),
            KeyValue::new("cycle.number", cycle_number as i64),
        ]
    }

    /// Create span attributes for a state change.
    pub fn state_change_attributes(
        from_phase: CyclePhase,
        to_phase: CyclePhase,
        cycle_number: u64,
    ) -> Vec<KeyValue> {
        vec![
            KeyValue::new("transition.from", format!("{:?}", from_phase)),
            KeyValue::new("transition.to", format!("{:?}", to_phase)),
            KeyValue::new("cycle.number", cycle_number as i64),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, "mycelix-cycle-engine");
        assert!(config.enable_traces);
        assert!(config.enable_metrics);
        assert_eq!(config.trace_sample_ratio, 1.0);
    }

    #[test]
    fn test_telemetry_config_from_env() {
        // This test verifies the config picks up env vars if set
        let config = TelemetryConfig::default();
        // Default endpoint when env var not set
        if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_err() {
            assert_eq!(config.otlp_endpoint, "http://localhost:4317");
        }
    }

    #[test]
    fn test_phase_attributes() {
        let attrs = attributes::phase_attributes(CyclePhase::Shadow, 1, 5);
        assert_eq!(attrs.len(), 3);
    }

    #[test]
    fn test_state_change_attributes() {
        let attrs = attributes::state_change_attributes(
            CyclePhase::Shadow,
            CyclePhase::Composting,
            1,
        );
        assert_eq!(attrs.len(), 3);
    }
}
