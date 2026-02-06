//! Metrics export plugin for the Metabolism Cycle Engine.
//!
//! Exports cycle engine metrics to external monitoring systems like
//! Prometheus, StatsD, or custom JSON endpoints.
//!
//! # Configuration
//!
//! ```toml
//! [config.settings]
//! export_format = "prometheus"
//! endpoint = "http://localhost:9091/metrics/job/mycelix"
//! export_interval_secs = 60
//! include_phase_metrics = true
//! include_event_counts = true
//! metric_prefix = "mycelix_cycle"
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use cycle_engine::plugin::{
    LogLevel, Plugin, PluginConfig, PluginContext, PluginError, PluginPriority, PluginStatus,
};
use living_core::{CyclePhase, LivingProtocolEvent};

/// Configuration for the metrics export plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsExportConfig {
    /// Export format
    #[serde(default = "default_format")]
    pub export_format: String,
    /// Endpoint URL for push-based exports
    #[serde(default)]
    pub endpoint: String,
    /// Export interval in seconds
    #[serde(default = "default_interval")]
    pub export_interval_secs: u64,
    /// Include phase-specific metrics
    #[serde(default = "default_true")]
    pub include_phase_metrics: bool,
    /// Include event counts
    #[serde(default = "default_true")]
    pub include_event_counts: bool,
    /// Metric prefix
    #[serde(default = "default_prefix")]
    pub metric_prefix: String,
    /// Additional labels
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

fn default_format() -> String {
    "prometheus".to_string()
}

fn default_interval() -> u64 {
    60
}

fn default_true() -> bool {
    true
}

fn default_prefix() -> String {
    "mycelix_cycle".to_string()
}

impl Default for MetricsExportConfig {
    fn default() -> Self {
        Self {
            export_format: default_format(),
            endpoint: String::new(),
            export_interval_secs: default_interval(),
            include_phase_metrics: true,
            include_event_counts: true,
            metric_prefix: default_prefix(),
            labels: HashMap::new(),
        }
    }
}

/// Collected metrics.
#[derive(Debug, Default)]
pub struct CollectedMetrics {
    // Counters
    pub phase_transitions: AtomicU64,
    pub cycles_started: AtomicU64,
    pub cycles_completed: AtomicU64,
    pub total_ticks: AtomicU64,
    pub total_events: AtomicU64,

    // Event counts by type
    pub event_counts: RwLock<HashMap<String, u64>>,

    // Phase metrics (latest snapshot)
    pub phase_metrics: RwLock<PhaseMetricsSnapshot>,

    // Timing
    pub last_tick_duration_ms: AtomicU64,
    pub last_export_time: RwLock<Option<DateTime<Utc>>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PhaseMetricsSnapshot {
    pub current_phase: String,
    pub cycle_number: u64,
    pub phase_day: u32,
    pub active_agents: u64,
    pub spectral_k: f64,
    pub mean_metabolic_trust: f64,
    pub active_wounds: u64,
    pub composting_entities: u64,
    pub liminal_entities: u64,
    pub entangled_pairs: u64,
    pub held_uncertainties: u64,
    pub captured_at: DateTime<Utc>,
}

/// The metrics export plugin.
pub struct MetricsExportPlugin {
    config: MetricsExportConfig,
    metrics: CollectedMetrics,
    last_tick: Option<Instant>,
    started_at: DateTime<Utc>,
}

impl MetricsExportPlugin {
    /// Create a new metrics export plugin.
    pub fn new() -> Self {
        Self {
            config: MetricsExportConfig::default(),
            metrics: CollectedMetrics::default(),
            last_tick: None,
            started_at: Utc::now(),
        }
    }

    /// Create with configuration.
    pub fn with_config(config: MetricsExportConfig) -> Self {
        Self {
            config,
            metrics: CollectedMetrics::default(),
            last_tick: None,
            started_at: Utc::now(),
        }
    }

    /// Get collected metrics.
    pub fn metrics(&self) -> &CollectedMetrics {
        &self.metrics
    }

    /// Export metrics in Prometheus format.
    pub fn export_prometheus(&self) -> String {
        let prefix = &self.config.metric_prefix;
        let labels = self.format_labels();

        let mut output = String::new();

        // Counters
        output.push_str(&format!(
            "# HELP {}_phase_transitions_total Total phase transitions\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_phase_transitions_total counter\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_phase_transitions_total{{{}}} {}\n",
            prefix,
            labels,
            self.metrics.phase_transitions.load(Ordering::Relaxed)
        ));

        output.push_str(&format!(
            "# HELP {}_cycles_started_total Total cycles started\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_cycles_started_total counter\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_cycles_started_total{{{}}} {}\n",
            prefix,
            labels,
            self.metrics.cycles_started.load(Ordering::Relaxed)
        ));

        output.push_str(&format!(
            "# HELP {}_cycles_completed_total Total cycles completed\n",
            prefix
        ));
        output.push_str(&format!(
            "# TYPE {}_cycles_completed_total counter\n",
            prefix
        ));
        output.push_str(&format!(
            "{}_cycles_completed_total{{{}}} {}\n",
            prefix,
            labels,
            self.metrics.cycles_completed.load(Ordering::Relaxed)
        ));

        output.push_str(&format!(
            "# HELP {}_ticks_total Total ticks processed\n",
            prefix
        ));
        output.push_str(&format!("# TYPE {}_ticks_total counter\n", prefix));
        output.push_str(&format!(
            "{}_ticks_total{{{}}} {}\n",
            prefix,
            labels,
            self.metrics.total_ticks.load(Ordering::Relaxed)
        ));

        output.push_str(&format!(
            "# HELP {}_events_total Total events processed\n",
            prefix
        ));
        output.push_str(&format!("# TYPE {}_events_total counter\n", prefix));
        output.push_str(&format!(
            "{}_events_total{{{}}} {}\n",
            prefix,
            labels,
            self.metrics.total_events.load(Ordering::Relaxed)
        ));

        // Phase metrics
        if self.config.include_phase_metrics {
            let snapshot = self.metrics.phase_metrics.read().unwrap();

            output.push_str(&format!(
                "# HELP {}_cycle_number Current cycle number\n",
                prefix
            ));
            output.push_str(&format!("# TYPE {}_cycle_number gauge\n", prefix));
            output.push_str(&format!(
                "{}_cycle_number{{{}}} {}\n",
                prefix, labels, snapshot.cycle_number
            ));

            output.push_str(&format!(
                "# HELP {}_phase_day Current day within phase\n",
                prefix
            ));
            output.push_str(&format!("# TYPE {}_phase_day gauge\n", prefix));
            output.push_str(&format!(
                "{}_phase_day{{{},phase=\"{}\"}} {}\n",
                prefix, labels, snapshot.current_phase, snapshot.phase_day
            ));

            output.push_str(&format!(
                "# HELP {}_active_agents Number of active agents\n",
                prefix
            ));
            output.push_str(&format!("# TYPE {}_active_agents gauge\n", prefix));
            output.push_str(&format!(
                "{}_active_agents{{{}}} {}\n",
                prefix, labels, snapshot.active_agents
            ));

            output.push_str(&format!(
                "# HELP {}_spectral_k Spectral K value\n",
                prefix
            ));
            output.push_str(&format!("# TYPE {}_spectral_k gauge\n", prefix));
            output.push_str(&format!(
                "{}_spectral_k{{{}}} {}\n",
                prefix, labels, snapshot.spectral_k
            ));

            output.push_str(&format!(
                "# HELP {}_metabolic_trust Mean metabolic trust score\n",
                prefix
            ));
            output.push_str(&format!("# TYPE {}_metabolic_trust gauge\n", prefix));
            output.push_str(&format!(
                "{}_metabolic_trust{{{}}} {}\n",
                prefix, labels, snapshot.mean_metabolic_trust
            ));

            output.push_str(&format!(
                "# HELP {}_active_wounds Number of active wounds\n",
                prefix
            ));
            output.push_str(&format!("# TYPE {}_active_wounds gauge\n", prefix));
            output.push_str(&format!(
                "{}_active_wounds{{{}}} {}\n",
                prefix, labels, snapshot.active_wounds
            ));

            output.push_str(&format!(
                "# HELP {}_composting_entities Number of entities being composted\n",
                prefix
            ));
            output.push_str(&format!("# TYPE {}_composting_entities gauge\n", prefix));
            output.push_str(&format!(
                "{}_composting_entities{{{}}} {}\n",
                prefix, labels, snapshot.composting_entities
            ));

            output.push_str(&format!(
                "# HELP {}_entangled_pairs Number of entangled pairs\n",
                prefix
            ));
            output.push_str(&format!("# TYPE {}_entangled_pairs gauge\n", prefix));
            output.push_str(&format!(
                "{}_entangled_pairs{{{}}} {}\n",
                prefix, labels, snapshot.entangled_pairs
            ));
        }

        // Event counts by type
        if self.config.include_event_counts {
            let event_counts = self.metrics.event_counts.read().unwrap();
            output.push_str(&format!(
                "# HELP {}_events_by_type_total Event counts by type\n",
                prefix
            ));
            output.push_str(&format!(
                "# TYPE {}_events_by_type_total counter\n",
                prefix
            ));
            for (event_type, count) in event_counts.iter() {
                output.push_str(&format!(
                    "{}_events_by_type_total{{{},event_type=\"{}\"}} {}\n",
                    prefix, labels, event_type, count
                ));
            }
        }

        output
    }

    /// Export metrics in JSON format.
    pub fn export_json(&self) -> serde_json::Value {
        let snapshot = self.metrics.phase_metrics.read().unwrap();
        let event_counts = self.metrics.event_counts.read().unwrap();

        serde_json::json!({
            "counters": {
                "phase_transitions": self.metrics.phase_transitions.load(Ordering::Relaxed),
                "cycles_started": self.metrics.cycles_started.load(Ordering::Relaxed),
                "cycles_completed": self.metrics.cycles_completed.load(Ordering::Relaxed),
                "total_ticks": self.metrics.total_ticks.load(Ordering::Relaxed),
                "total_events": self.metrics.total_events.load(Ordering::Relaxed),
            },
            "phase_metrics": {
                "current_phase": snapshot.current_phase,
                "cycle_number": snapshot.cycle_number,
                "phase_day": snapshot.phase_day,
                "active_agents": snapshot.active_agents,
                "spectral_k": snapshot.spectral_k,
                "mean_metabolic_trust": snapshot.mean_metabolic_trust,
                "active_wounds": snapshot.active_wounds,
                "composting_entities": snapshot.composting_entities,
                "liminal_entities": snapshot.liminal_entities,
                "entangled_pairs": snapshot.entangled_pairs,
                "held_uncertainties": snapshot.held_uncertainties,
            },
            "event_counts": *event_counts,
            "labels": self.config.labels,
            "exported_at": Utc::now().to_rfc3339(),
        })
    }

    /// Export metrics in StatsD format.
    pub fn export_statsd(&self) -> String {
        let prefix = &self.config.metric_prefix;
        let mut output = String::new();

        // Counters (using gauge type for simplicity)
        output.push_str(&format!(
            "{}.phase_transitions:{}|g\n",
            prefix,
            self.metrics.phase_transitions.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "{}.cycles_started:{}|g\n",
            prefix,
            self.metrics.cycles_started.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "{}.total_ticks:{}|g\n",
            prefix,
            self.metrics.total_ticks.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "{}.total_events:{}|g\n",
            prefix,
            self.metrics.total_events.load(Ordering::Relaxed)
        ));

        // Phase metrics
        let snapshot = self.metrics.phase_metrics.read().unwrap();
        output.push_str(&format!(
            "{}.cycle_number:{}|g\n",
            prefix, snapshot.cycle_number
        ));
        output.push_str(&format!(
            "{}.active_agents:{}|g\n",
            prefix, snapshot.active_agents
        ));
        output.push_str(&format!("{}.spectral_k:{}|g\n", prefix, snapshot.spectral_k));

        output
    }

    /// Format labels for Prometheus.
    fn format_labels(&self) -> String {
        self.config
            .labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Update phase metrics snapshot.
    fn update_phase_metrics(&self, ctx: &PluginContext) {
        let metrics = ctx.metrics();
        let mut snapshot = self.metrics.phase_metrics.write().unwrap();

        *snapshot = PhaseMetricsSnapshot {
            current_phase: format!("{:?}", ctx.current_phase()),
            cycle_number: ctx.cycle_number(),
            phase_day: ctx.phase_day(),
            active_agents: metrics.active_agents,
            spectral_k: metrics.spectral_k,
            mean_metabolic_trust: metrics.mean_metabolic_trust,
            active_wounds: metrics.active_wounds,
            composting_entities: metrics.composting_entities,
            liminal_entities: metrics.liminal_entities,
            entangled_pairs: metrics.entangled_pairs,
            held_uncertainties: metrics.held_uncertainties,
            captured_at: Utc::now(),
        };
    }

    /// Get the event type name.
    fn event_type_name(&self, event: &LivingProtocolEvent) -> String {
        match event {
            LivingProtocolEvent::CompostingStarted(_) => "composting_started",
            LivingProtocolEvent::NutrientExtracted(_) => "nutrient_extracted",
            LivingProtocolEvent::CompostingCompleted(_) => "composting_completed",
            LivingProtocolEvent::WoundCreated(_) => "wound_created",
            LivingProtocolEvent::WoundPhaseAdvanced(_) => "wound_phase_advanced",
            LivingProtocolEvent::RestitutionFulfilled(_) => "restitution_fulfilled",
            LivingProtocolEvent::ScarTissueFormed(_) => "scar_tissue_formed",
            LivingProtocolEvent::MetabolicTrustUpdated(_) => "metabolic_trust_updated",
            LivingProtocolEvent::KenosisCommitted(_) => "kenosis_committed",
            LivingProtocolEvent::KenosisExecuted(_) => "kenosis_executed",
            LivingProtocolEvent::TemporalKVectorUpdated(_) => "temporal_kvector_updated",
            LivingProtocolEvent::FieldInterferenceDetected(_) => "field_interference_detected",
            LivingProtocolEvent::DreamStateChanged(_) => "dream_state_changed",
            LivingProtocolEvent::DreamProposalGenerated(_) => "dream_proposal_generated",
            LivingProtocolEvent::NetworkPhiComputed(_) => "network_phi_computed",
            LivingProtocolEvent::ShadowSurfaced(_) => "shadow_surfaced",
            LivingProtocolEvent::ClaimHeldInUncertainty(_) => "claim_held",
            LivingProtocolEvent::ClaimReleasedFromUncertainty(_) => "claim_released",
            LivingProtocolEvent::SilenceDetected(_) => "silence_detected",
            LivingProtocolEvent::BeautyScored(_) => "beauty_scored",
            LivingProtocolEvent::EntanglementFormed(_) => "entanglement_formed",
            LivingProtocolEvent::EntanglementDecayed(_) => "entanglement_decayed",
            LivingProtocolEvent::AttractorFieldComputed(_) => "attractor_field_computed",
            LivingProtocolEvent::LiminalTransitionStarted(_) => "liminal_transition_started",
            LivingProtocolEvent::LiminalTransitionCompleted(_) => "liminal_transition_completed",
            LivingProtocolEvent::InterSpeciesRegistered(_) => "inter_species_registered",
            LivingProtocolEvent::ResonanceAddressCreated(_) => "resonance_address_created",
            LivingProtocolEvent::FractalPatternReplicated(_) => "fractal_pattern_replicated",
            LivingProtocolEvent::MorphogeneticFieldUpdated(_) => "morphogenetic_field_updated",
            LivingProtocolEvent::TimeCrystalPeriodStarted(_) => "time_crystal_period_started",
            LivingProtocolEvent::MycelialTaskDistributed(_) => "mycelial_task_distributed",
            LivingProtocolEvent::MycelialTaskCompleted(_) => "mycelial_task_completed",
            LivingProtocolEvent::PhaseTransitioned(_) => "phase_transitioned",
            LivingProtocolEvent::CycleStarted(_) => "cycle_started",
        }
        .to_string()
    }
}

impl Default for MetricsExportPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MetricsExportPlugin {
    fn name(&self) -> &str {
        "metrics-export"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn priority(&self) -> PluginPriority {
        // Run after other plugins to capture accurate metrics
        PluginPriority::Low
    }

    fn on_load(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError> {
        self.update_phase_metrics(ctx);
        info!(
            plugin = self.name(),
            format = %self.config.export_format,
            "Metrics export plugin loaded"
        );
        Ok(())
    }

    fn on_phase_enter(&self, _phase: CyclePhase, ctx: &mut PluginContext) {
        self.update_phase_metrics(ctx);
    }

    fn on_phase_exit(&self, _phase: CyclePhase, ctx: &mut PluginContext) {
        self.update_phase_metrics(ctx);
    }

    fn on_tick(&self, ctx: &mut PluginContext) {
        self.metrics.total_ticks.fetch_add(1, Ordering::Relaxed);
        self.update_phase_metrics(ctx);
    }

    fn on_event(&self, event: &LivingProtocolEvent, _ctx: &mut PluginContext) {
        self.metrics.total_events.fetch_add(1, Ordering::Relaxed);

        // Track by event type
        let event_type = self.event_type_name(event);
        let mut counts = self.metrics.event_counts.write().unwrap();
        *counts.entry(event_type).or_insert(0) += 1;
    }

    fn on_phase_transition(&self, _from: CyclePhase, _to: CyclePhase, ctx: &mut PluginContext) {
        self.metrics.phase_transitions.fetch_add(1, Ordering::Relaxed);
        self.update_phase_metrics(ctx);
    }

    fn on_cycle_start(&self, _cycle_number: u64, ctx: &mut PluginContext) {
        self.metrics.cycles_started.fetch_add(1, Ordering::Relaxed);
        self.update_phase_metrics(ctx);
    }

    fn on_cycle_complete(&self, _cycle_number: u64, ctx: &mut PluginContext) {
        self.metrics.cycles_completed.fetch_add(1, Ordering::Relaxed);
        self.update_phase_metrics(ctx);
    }

    fn configure(&mut self, config: PluginConfig) -> Result<(), PluginError> {
        if let Ok(export_config) = serde_json::from_value::<MetricsExportConfig>(config.settings) {
            self.config = export_config;
        }
        Ok(())
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "export_format": {
                    "type": "string",
                    "enum": ["prometheus", "statsd", "json"],
                    "default": "prometheus"
                },
                "endpoint": {
                    "type": "string",
                    "description": "Push endpoint URL"
                },
                "export_interval_secs": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 60
                },
                "include_phase_metrics": {
                    "type": "boolean",
                    "default": true
                },
                "include_event_counts": {
                    "type": "boolean",
                    "default": true
                },
                "metric_prefix": {
                    "type": "string",
                    "default": "mycelix_cycle"
                },
                "labels": {
                    "type": "object",
                    "additionalProperties": { "type": "string" }
                }
            }
        }))
    }

    fn status(&self) -> PluginStatus {
        PluginStatus::Running
    }

    fn metrics(&self) -> serde_json::Value {
        self.export_json()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Entry point for dynamic loading.
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(MetricsExportPlugin::new());
    Box::into_raw(plugin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use living_core::{CycleStartedEvent, CycleState, PhaseMetrics};

    fn test_context() -> PluginContext {
        PluginContext::new(
            CycleState {
                cycle_number: 5,
                current_phase: CyclePhase::CoCreation,
                phase_started: Utc::now(),
                cycle_started: Utc::now(),
                phase_day: 3,
            },
            PhaseMetrics {
                active_agents: 100,
                spectral_k: 0.75,
                mean_metabolic_trust: 0.82,
                active_wounds: 5,
                composting_entities: 3,
                liminal_entities: 8,
                entangled_pairs: 42,
                held_uncertainties: 7,
            },
        )
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = MetricsExportPlugin::new();
        assert_eq!(plugin.name(), "metrics-export");
    }

    #[test]
    fn test_prometheus_export() {
        let plugin = MetricsExportPlugin::new();
        let output = plugin.export_prometheus();

        assert!(output.contains("mycelix_cycle_phase_transitions_total"));
        assert!(output.contains("mycelix_cycle_cycles_started_total"));
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
    }

    #[test]
    fn test_json_export() {
        let plugin = MetricsExportPlugin::new();
        let output = plugin.export_json();

        assert!(output["counters"]["phase_transitions"].is_number());
        assert!(output["phase_metrics"]["current_phase"].is_string());
    }

    #[test]
    fn test_event_counting() {
        let mut plugin = MetricsExportPlugin::new();
        let mut ctx = test_context();

        plugin.on_load(&mut ctx).unwrap();

        let event = LivingProtocolEvent::CycleStarted(CycleStartedEvent {
            cycle_number: 1,
            started_at: Utc::now(),
        });

        plugin.on_event(&event, &mut ctx);
        plugin.on_event(&event, &mut ctx);

        assert_eq!(plugin.metrics.total_events.load(Ordering::Relaxed), 2);

        let counts = plugin.metrics.event_counts.read().unwrap();
        assert_eq!(counts.get("cycle_started"), Some(&2));
    }

    #[test]
    fn test_phase_metrics_update() {
        let mut plugin = MetricsExportPlugin::new();
        let mut ctx = test_context();

        plugin.on_load(&mut ctx).unwrap();
        plugin.on_tick(&mut ctx);

        let snapshot = plugin.metrics.phase_metrics.read().unwrap();
        assert_eq!(snapshot.cycle_number, 5);
        assert_eq!(snapshot.active_agents, 100);
        assert_eq!(snapshot.current_phase, "CoCreation");
    }
}
