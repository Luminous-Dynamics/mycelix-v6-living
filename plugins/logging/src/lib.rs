//! Detailed phase transition logging plugin for the Metabolism Cycle Engine.
//!
//! This plugin provides comprehensive logging of:
//! - Phase transitions with timing information
//! - Phase metrics at entry/exit
//! - Living Protocol events
//! - Cycle lifecycle events
//!
//! # Configuration
//!
//! ```toml
//! [config.settings]
//! log_level = "info"
//! log_metrics = true
//! log_events = true
//! include_timestamps = true
//! output_format = "text"
//! ```

use std::any::Any;
use std::collections::VecDeque;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};

use cycle_engine::plugin::{
    LogLevel, Plugin, PluginConfig, PluginContext, PluginError, PluginPriority, PluginStatus,
};
use living_core::{CyclePhase, LivingProtocolEvent};

/// Configuration for the logging plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level threshold
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Whether to log phase metrics
    #[serde(default = "default_true")]
    pub log_metrics: bool,
    /// Whether to log event details
    #[serde(default = "default_true")]
    pub log_events: bool,
    /// Whether to include timestamps
    #[serde(default = "default_true")]
    pub include_timestamps: bool,
    /// Output format ("text" or "json")
    #[serde(default = "default_format")]
    pub output_format: String,
    /// Maximum number of recent events to keep in memory
    #[serde(default = "default_max_events")]
    pub max_recent_events: usize,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

fn default_format() -> String {
    "text".to_string()
}

fn default_max_events() -> usize {
    1000
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            log_metrics: true,
            log_events: true,
            include_timestamps: true,
            output_format: default_format(),
            max_recent_events: default_max_events(),
        }
    }
}

/// A logged event record.
#[derive(Debug, Clone, Serialize)]
pub struct LogRecord {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub category: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// The logging plugin implementation.
pub struct LoggingPlugin {
    config: LoggingConfig,
    /// Recent log records (for debugging/inspection)
    recent_logs: VecDeque<LogRecord>,
    /// Phase entry timestamps for duration tracking
    phase_entry_times: std::collections::HashMap<CyclePhase, Instant>,
    /// Statistics
    stats: LoggingStats,
}

#[derive(Debug, Clone, Default, Serialize)]
struct LoggingStats {
    total_logs: u64,
    phase_entries: u64,
    phase_exits: u64,
    events_logged: u64,
    cycles_started: u64,
    cycles_completed: u64,
}

impl LoggingPlugin {
    /// Create a new logging plugin with default configuration.
    pub fn new() -> Self {
        Self {
            config: LoggingConfig::default(),
            recent_logs: VecDeque::new(),
            phase_entry_times: std::collections::HashMap::new(),
            stats: LoggingStats::default(),
        }
    }

    /// Create a new logging plugin with the given configuration.
    pub fn with_config(config: LoggingConfig) -> Self {
        Self {
            config,
            recent_logs: VecDeque::new(),
            phase_entry_times: std::collections::HashMap::new(),
            stats: LoggingStats::default(),
        }
    }

    /// Log a record.
    fn log(&mut self, level: &str, category: &str, message: &str, data: Option<serde_json::Value>) {
        let record = LogRecord {
            timestamp: Utc::now(),
            level: level.to_string(),
            category: category.to_string(),
            message: message.to_string(),
            data,
        };

        // Output based on format
        if self.config.output_format == "json" {
            match level {
                "trace" => trace!("{}", serde_json::to_string(&record).unwrap_or_default()),
                "debug" => debug!("{}", serde_json::to_string(&record).unwrap_or_default()),
                "info" => info!("{}", serde_json::to_string(&record).unwrap_or_default()),
                "warn" => warn!("{}", serde_json::to_string(&record).unwrap_or_default()),
                "error" => error!("{}", serde_json::to_string(&record).unwrap_or_default()),
                _ => info!("{}", serde_json::to_string(&record).unwrap_or_default()),
            }
        } else {
            let timestamp_prefix = if self.config.include_timestamps {
                format!("[{}] ", record.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"))
            } else {
                String::new()
            };

            let data_suffix = if let Some(ref d) = record.data {
                format!(" | {}", d)
            } else {
                String::new()
            };

            let formatted = format!(
                "{}[{}] {}{}",
                timestamp_prefix, category, message, data_suffix
            );

            match level {
                "trace" => trace!("{}", formatted),
                "debug" => debug!("{}", formatted),
                "info" => info!("{}", formatted),
                "warn" => warn!("{}", formatted),
                "error" => error!("{}", formatted),
                _ => info!("{}", formatted),
            }
        }

        // Store in recent logs
        self.recent_logs.push_back(record);
        while self.recent_logs.len() > self.config.max_recent_events {
            self.recent_logs.pop_front();
        }

        self.stats.total_logs += 1;
    }

    /// Get recent logs.
    pub fn recent_logs(&self) -> &VecDeque<LogRecord> {
        &self.recent_logs
    }

    /// Get statistics.
    pub fn stats(&self) -> &LoggingStats {
        &self.stats
    }

    /// Format a phase for logging.
    fn format_phase(&self, phase: CyclePhase) -> String {
        format!("{:?}", phase)
    }

    /// Format metrics for logging.
    fn format_metrics(&self, ctx: &PluginContext) -> serde_json::Value {
        let metrics = ctx.metrics();
        serde_json::json!({
            "active_agents": metrics.active_agents,
            "spectral_k": metrics.spectral_k,
            "mean_metabolic_trust": metrics.mean_metabolic_trust,
            "active_wounds": metrics.active_wounds,
            "composting_entities": metrics.composting_entities,
            "liminal_entities": metrics.liminal_entities,
            "entangled_pairs": metrics.entangled_pairs,
            "held_uncertainties": metrics.held_uncertainties,
        })
    }

    /// Format an event for logging.
    fn format_event(&self, event: &LivingProtocolEvent) -> (String, serde_json::Value) {
        match event {
            LivingProtocolEvent::PhaseTransitioned(e) => (
                "Phase transition".to_string(),
                serde_json::json!({
                    "from": format!("{:?}", e.transition.from),
                    "to": format!("{:?}", e.transition.to),
                    "cycle": e.transition.cycle_number,
                }),
            ),
            LivingProtocolEvent::CycleStarted(e) => (
                "Cycle started".to_string(),
                serde_json::json!({
                    "cycle_number": e.cycle_number,
                    "started_at": e.started_at.to_rfc3339(),
                }),
            ),
            LivingProtocolEvent::CompostingStarted(e) => (
                "Composting started".to_string(),
                serde_json::json!({
                    "record_id": e.record_id,
                    "entity_type": format!("{:?}", e.entity_type),
                    "entity_id": e.entity_id,
                }),
            ),
            LivingProtocolEvent::WoundCreated(e) => (
                "Wound created".to_string(),
                serde_json::json!({
                    "wound_id": e.wound_id,
                    "agent_did": e.agent_did,
                    "severity": format!("{:?}", e.severity),
                }),
            ),
            LivingProtocolEvent::KenosisCommitted(e) => (
                "Kenosis committed".to_string(),
                serde_json::json!({
                    "commitment_id": e.commitment_id,
                    "agent_did": e.agent_did,
                    "release_percentage": e.release_percentage,
                }),
            ),
            LivingProtocolEvent::ShadowSurfaced(e) => (
                "Shadow surfaced".to_string(),
                serde_json::json!({
                    "shadow_id": e.shadow.id,
                    "original_content_id": e.shadow.original_content_id,
                    "low_rep_dissent": e.shadow.low_rep_dissent,
                }),
            ),
            LivingProtocolEvent::ClaimHeldInUncertainty(e) => (
                "Claim held in uncertainty".to_string(),
                serde_json::json!({
                    "claim_id": e.claim_id,
                    "reason": e.reason,
                }),
            ),
            LivingProtocolEvent::EntanglementFormed(e) => (
                "Entanglement formed".to_string(),
                serde_json::json!({
                    "pair_id": e.pair.id,
                    "agent_a": e.pair.agent_a,
                    "agent_b": e.pair.agent_b,
                    "strength": e.pair.entanglement_strength,
                }),
            ),
            LivingProtocolEvent::DreamStateChanged(e) => (
                "Dream state changed".to_string(),
                serde_json::json!({
                    "from": format!("{:?}", e.from),
                    "to": format!("{:?}", e.to),
                    "participation": e.network_participation,
                }),
            ),
            // Default case for other events
            _ => (
                format!("{:?}", std::mem::discriminant(event)),
                serde_json::json!({}),
            ),
        }
    }
}

impl Default for LoggingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for LoggingPlugin {
    fn name(&self) -> &str {
        "logging"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn priority(&self) -> PluginPriority {
        // Run early to capture all events
        PluginPriority::High
    }

    fn on_load(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError> {
        self.log(
            "info",
            "lifecycle",
            &format!(
                "Logging plugin loaded (cycle {}, phase {:?})",
                ctx.cycle_number(),
                ctx.current_phase()
            ),
            None,
        );
        Ok(())
    }

    fn on_unload(&mut self, _ctx: &mut PluginContext) -> Result<(), PluginError> {
        self.log(
            "info",
            "lifecycle",
            &format!(
                "Logging plugin unloading (total logs: {})",
                self.stats.total_logs
            ),
            None,
        );
        Ok(())
    }

    fn on_phase_enter(&self, phase: CyclePhase, ctx: &mut PluginContext) {
        // Use interior mutability workaround via ctx metadata
        let message = format!(
            "Entering {} phase (cycle {}, day {})",
            self.format_phase(phase),
            ctx.cycle_number(),
            ctx.phase_day()
        );

        let data = if self.config.log_metrics {
            Some(self.format_metrics(ctx))
        } else {
            None
        };

        // Log via context since we can't mutate self
        ctx.log(LogLevel::Info, format!("[phase_entry] {} {:?}", message, data));
    }

    fn on_phase_exit(&self, phase: CyclePhase, ctx: &mut PluginContext) {
        let message = format!(
            "Exiting {} phase (cycle {}, day {})",
            self.format_phase(phase),
            ctx.cycle_number(),
            ctx.phase_day()
        );

        let data = if self.config.log_metrics {
            Some(self.format_metrics(ctx))
        } else {
            None
        };

        ctx.log(LogLevel::Info, format!("[phase_exit] {} {:?}", message, data));
    }

    fn on_tick(&self, ctx: &mut PluginContext) {
        if self.config.log_level == "trace" {
            ctx.log(
                LogLevel::Trace,
                format!(
                    "[tick] Phase {:?}, day {}, cycle {}",
                    ctx.current_phase(),
                    ctx.phase_day(),
                    ctx.cycle_number()
                ),
            );
        }
    }

    fn on_event(&self, event: &LivingProtocolEvent, ctx: &mut PluginContext) {
        if !self.config.log_events {
            return;
        }

        let (message, data) = self.format_event(event);
        ctx.log(LogLevel::Info, format!("[event] {} {:?}", message, data));
    }

    fn on_phase_transition(&self, from: CyclePhase, to: CyclePhase, ctx: &mut PluginContext) {
        ctx.log(
            LogLevel::Info,
            format!(
                "[transition] {:?} -> {:?} (cycle {})",
                from,
                to,
                ctx.cycle_number()
            ),
        );
    }

    fn on_cycle_start(&self, cycle_number: u64, ctx: &mut PluginContext) {
        ctx.log(
            LogLevel::Info,
            format!("[cycle_start] Cycle {} started", cycle_number),
        );
    }

    fn on_cycle_complete(&self, cycle_number: u64, ctx: &mut PluginContext) {
        ctx.log(
            LogLevel::Info,
            format!("[cycle_complete] Cycle {} completed", cycle_number),
        );
    }

    fn configure(&mut self, config: PluginConfig) -> Result<(), PluginError> {
        if let Ok(logging_config) = serde_json::from_value::<LoggingConfig>(config.settings) {
            self.config = logging_config;
        }
        Ok(())
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "log_level": {
                    "type": "string",
                    "enum": ["trace", "debug", "info", "warn", "error"],
                    "default": "info"
                },
                "log_metrics": {
                    "type": "boolean",
                    "default": true
                },
                "log_events": {
                    "type": "boolean",
                    "default": true
                },
                "include_timestamps": {
                    "type": "boolean",
                    "default": true
                },
                "output_format": {
                    "type": "string",
                    "enum": ["text", "json"],
                    "default": "text"
                },
                "max_recent_events": {
                    "type": "integer",
                    "minimum": 0,
                    "default": 1000
                }
            }
        }))
    }

    fn status(&self) -> PluginStatus {
        PluginStatus::Running
    }

    fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "total_logs": self.stats.total_logs,
            "phase_entries": self.stats.phase_entries,
            "phase_exits": self.stats.phase_exits,
            "events_logged": self.stats.events_logged,
            "cycles_started": self.stats.cycles_started,
            "cycles_completed": self.stats.cycles_completed,
            "recent_logs_count": self.recent_logs.len(),
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Entry point for dynamic loading.
///
/// # Safety
///
/// This function is called by the dynamic plugin loader.
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(LoggingPlugin::new());
    Box::into_raw(plugin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use living_core::{CycleState, PhaseMetrics};

    fn test_context() -> PluginContext {
        PluginContext::new(
            CycleState {
                cycle_number: 1,
                current_phase: CyclePhase::Shadow,
                phase_started: Utc::now(),
                cycle_started: Utc::now(),
                phase_day: 0,
            },
            PhaseMetrics {
                active_agents: 10,
                spectral_k: 0.5,
                mean_metabolic_trust: 0.7,
                active_wounds: 2,
                composting_entities: 1,
                liminal_entities: 3,
                entangled_pairs: 5,
                held_uncertainties: 2,
            },
        )
    }

    #[test]
    fn test_logging_plugin_creation() {
        let plugin = LoggingPlugin::new();
        assert_eq!(plugin.name(), "logging");
        assert_eq!(plugin.priority(), PluginPriority::High);
    }

    #[test]
    fn test_logging_plugin_on_load() {
        let mut plugin = LoggingPlugin::new();
        let mut ctx = test_context();

        plugin.on_load(&mut ctx).unwrap();
        assert!(plugin.stats.total_logs > 0);
    }

    #[test]
    fn test_logging_config() {
        let config = LoggingConfig {
            log_level: "debug".to_string(),
            log_metrics: false,
            log_events: true,
            include_timestamps: false,
            output_format: "json".to_string(),
            max_recent_events: 500,
        };

        let plugin = LoggingPlugin::with_config(config.clone());
        assert_eq!(plugin.config.log_level, "debug");
        assert!(!plugin.config.log_metrics);
    }

    #[test]
    fn test_format_event() {
        let plugin = LoggingPlugin::new();

        let event = LivingProtocolEvent::CycleStarted(living_core::CycleStartedEvent {
            cycle_number: 5,
            started_at: Utc::now(),
        });

        let (message, data) = plugin.format_event(&event);
        assert_eq!(message, "Cycle started");
        assert_eq!(data["cycle_number"], 5);
    }
}
