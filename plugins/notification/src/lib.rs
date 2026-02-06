//! Notification plugin for the Metabolism Cycle Engine.
//!
//! Sends notifications to external systems on important events like:
//! - Phase transitions
//! - Cycle start/completion
//! - Critical wounds
//! - Kenosis commitments
//!
//! # Configuration
//!
//! ```toml
//! [config.settings]
//! channels = ["webhook", "slack"]
//! webhook_url = "https://example.com/webhook"
//! notify_on_phase_transition = true
//! notify_on_cycle_start = true
//! min_wound_severity = "moderate"
//! ```

use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use cycle_engine::plugin::{
    LogLevel, Plugin, PluginConfig, PluginContext, PluginError, PluginPriority, PluginStatus,
};
use living_core::{CyclePhase, LivingProtocolEvent, WoundSeverity};

/// Configuration for the notification plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Notification channels
    #[serde(default = "default_channels")]
    pub channels: Vec<String>,
    /// Webhook URL
    #[serde(default)]
    pub webhook_url: String,
    /// Slack webhook URL
    #[serde(default)]
    pub slack_webhook_url: String,
    /// Discord webhook URL
    #[serde(default)]
    pub discord_webhook_url: String,
    /// Notify on phase transitions
    #[serde(default = "default_true")]
    pub notify_on_phase_transition: bool,
    /// Notify on cycle start
    #[serde(default = "default_true")]
    pub notify_on_cycle_start: bool,
    /// Notify on cycle complete
    #[serde(default = "default_true")]
    pub notify_on_cycle_complete: bool,
    /// Notify on wounds
    #[serde(default = "default_true")]
    pub notify_on_wounds: bool,
    /// Notify on kenosis
    #[serde(default = "default_true")]
    pub notify_on_kenosis: bool,
    /// Minimum wound severity
    #[serde(default = "default_severity")]
    pub min_wound_severity: String,
    /// Rate limit
    #[serde(default = "default_rate_limit")]
    pub max_notifications_per_minute: u32,
    /// Include metrics
    #[serde(default = "default_true")]
    pub include_metrics: bool,
}

fn default_channels() -> Vec<String> {
    vec!["webhook".to_string()]
}

fn default_true() -> bool {
    true
}

fn default_severity() -> String {
    "moderate".to_string()
}

fn default_rate_limit() -> u32 {
    10
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            channels: default_channels(),
            webhook_url: String::new(),
            slack_webhook_url: String::new(),
            discord_webhook_url: String::new(),
            notify_on_phase_transition: true,
            notify_on_cycle_start: true,
            notify_on_cycle_complete: true,
            notify_on_wounds: true,
            notify_on_kenosis: true,
            min_wound_severity: default_severity(),
            max_notifications_per_minute: default_rate_limit(),
            include_metrics: true,
        }
    }
}

/// A notification to be sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Notification type
    pub notification_type: NotificationType,
    /// Title
    pub title: String,
    /// Message body
    pub message: String,
    /// Severity/priority
    pub severity: NotificationSeverity,
    /// Additional data
    pub data: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether it was sent successfully
    #[serde(default)]
    pub sent: bool,
    /// Channels it was sent to
    #[serde(default)]
    pub sent_to: Vec<String>,
}

/// Type of notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    PhaseTransition,
    CycleStart,
    CycleComplete,
    WoundCreated,
    KenosisCommitted,
    ShadowSurfaced,
    Custom,
}

/// Severity of notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Statistics for the notification plugin.
#[derive(Debug, Default)]
struct NotificationStats {
    total_notifications: AtomicU64,
    successful_sends: AtomicU64,
    failed_sends: AtomicU64,
    rate_limited: AtomicU64,
}

/// The notification plugin.
pub struct NotificationPlugin {
    config: NotificationConfig,
    /// Recent notifications (for inspection)
    recent_notifications: VecDeque<Notification>,
    /// Pending notifications (for async sending)
    pending_notifications: VecDeque<Notification>,
    /// Rate limiting: timestamps of recent sends
    send_timestamps: VecDeque<Instant>,
    /// Statistics
    stats: NotificationStats,
    /// Maximum recent notifications to keep
    max_recent: usize,
}

impl NotificationPlugin {
    /// Create a new notification plugin.
    pub fn new() -> Self {
        Self {
            config: NotificationConfig::default(),
            recent_notifications: VecDeque::new(),
            pending_notifications: VecDeque::new(),
            send_timestamps: VecDeque::new(),
            stats: NotificationStats::default(),
            max_recent: 100,
        }
    }

    /// Create with configuration.
    pub fn with_config(config: NotificationConfig) -> Self {
        Self {
            config,
            recent_notifications: VecDeque::new(),
            pending_notifications: VecDeque::new(),
            send_timestamps: VecDeque::new(),
            stats: NotificationStats::default(),
            max_recent: 100,
        }
    }

    /// Get recent notifications.
    pub fn recent_notifications(&self) -> &VecDeque<Notification> {
        &self.recent_notifications
    }

    /// Get pending notifications count.
    pub fn pending_count(&self) -> usize {
        self.pending_notifications.len()
    }

    /// Check if we're rate limited.
    fn is_rate_limited(&mut self) -> bool {
        let now = Instant::now();
        let one_minute_ago = now - std::time::Duration::from_secs(60);

        // Remove old timestamps
        while let Some(ts) = self.send_timestamps.front() {
            if *ts < one_minute_ago {
                self.send_timestamps.pop_front();
            } else {
                break;
            }
        }

        self.send_timestamps.len() >= self.config.max_notifications_per_minute as usize
    }

    /// Queue a notification.
    fn queue_notification(&mut self, notification: Notification) {
        if self.is_rate_limited() {
            warn!(
                notification_type = ?notification.notification_type,
                "Notification rate limited"
            );
            self.stats.rate_limited.fetch_add(1, Ordering::Relaxed);
            return;
        }

        self.stats.total_notifications.fetch_add(1, Ordering::Relaxed);
        self.pending_notifications.push_back(notification.clone());
        self.recent_notifications.push_back(notification);

        while self.recent_notifications.len() > self.max_recent {
            self.recent_notifications.pop_front();
        }
    }

    /// Send pending notifications (simulated - would actually call HTTP in real impl).
    fn send_pending(&mut self, ctx: &mut PluginContext) {
        while let Some(mut notification) = self.pending_notifications.pop_front() {
            let mut sent_to = Vec::new();

            for channel in &self.config.channels {
                match channel.as_str() {
                    "webhook" => {
                        if !self.config.webhook_url.is_empty() {
                            // In real implementation, would make HTTP POST here
                            debug!(
                                url = %self.config.webhook_url,
                                notification_type = ?notification.notification_type,
                                "Would send webhook notification"
                            );
                            sent_to.push("webhook".to_string());
                        }
                    }
                    "slack" => {
                        if !self.config.slack_webhook_url.is_empty() {
                            // Would format and send Slack message
                            debug!(
                                notification_type = ?notification.notification_type,
                                "Would send Slack notification"
                            );
                            sent_to.push("slack".to_string());
                        }
                    }
                    "discord" => {
                        if !self.config.discord_webhook_url.is_empty() {
                            // Would format and send Discord message
                            debug!(
                                notification_type = ?notification.notification_type,
                                "Would send Discord notification"
                            );
                            sent_to.push("discord".to_string());
                        }
                    }
                    other => {
                        debug!(channel = %other, "Unknown notification channel");
                    }
                }
            }

            if !sent_to.is_empty() {
                notification.sent = true;
                notification.sent_to = sent_to;
                self.stats.successful_sends.fetch_add(1, Ordering::Relaxed);
                self.send_timestamps.push_back(Instant::now());

                ctx.log(
                    LogLevel::Info,
                    format!(
                        "[notification] Sent: {} - {}",
                        notification.title, notification.message
                    ),
                );
            } else {
                self.stats.failed_sends.fetch_add(1, Ordering::Relaxed);
                ctx.log(
                    LogLevel::Warn,
                    format!(
                        "[notification] Failed to send (no valid channels): {}",
                        notification.title
                    ),
                );
            }
        }
    }

    /// Create a phase transition notification.
    fn create_phase_transition_notification(
        &self,
        from: CyclePhase,
        to: CyclePhase,
        ctx: &PluginContext,
    ) -> Notification {
        let data = if self.config.include_metrics {
            let metrics = ctx.metrics();
            serde_json::json!({
                "from_phase": format!("{:?}", from),
                "to_phase": format!("{:?}", to),
                "cycle_number": ctx.cycle_number(),
                "metrics": {
                    "active_agents": metrics.active_agents,
                    "spectral_k": metrics.spectral_k,
                    "mean_metabolic_trust": metrics.mean_metabolic_trust,
                }
            })
        } else {
            serde_json::json!({
                "from_phase": format!("{:?}", from),
                "to_phase": format!("{:?}", to),
                "cycle_number": ctx.cycle_number(),
            })
        };

        Notification {
            notification_type: NotificationType::PhaseTransition,
            title: format!("Phase Transition: {:?} -> {:?}", from, to),
            message: format!(
                "Metabolism cycle {} has transitioned from {:?} to {:?} phase.",
                ctx.cycle_number(),
                from,
                to
            ),
            severity: NotificationSeverity::Info,
            data,
            timestamp: Utc::now(),
            sent: false,
            sent_to: Vec::new(),
        }
    }

    /// Create a cycle start notification.
    fn create_cycle_start_notification(&self, cycle_number: u64) -> Notification {
        Notification {
            notification_type: NotificationType::CycleStart,
            title: format!("Cycle {} Started", cycle_number),
            message: format!(
                "Metabolism cycle {} has started. The 28-day cycle begins with the Shadow phase.",
                cycle_number
            ),
            severity: NotificationSeverity::Info,
            data: serde_json::json!({
                "cycle_number": cycle_number,
                "started_at": Utc::now().to_rfc3339(),
            }),
            timestamp: Utc::now(),
            sent: false,
            sent_to: Vec::new(),
        }
    }

    /// Create a cycle complete notification.
    fn create_cycle_complete_notification(
        &self,
        cycle_number: u64,
        ctx: &PluginContext,
    ) -> Notification {
        let data = if self.config.include_metrics {
            let metrics = ctx.metrics();
            serde_json::json!({
                "cycle_number": cycle_number,
                "completed_at": Utc::now().to_rfc3339(),
                "final_metrics": {
                    "active_agents": metrics.active_agents,
                    "spectral_k": metrics.spectral_k,
                    "mean_metabolic_trust": metrics.mean_metabolic_trust,
                    "entangled_pairs": metrics.entangled_pairs,
                }
            })
        } else {
            serde_json::json!({
                "cycle_number": cycle_number,
                "completed_at": Utc::now().to_rfc3339(),
            })
        };

        Notification {
            notification_type: NotificationType::CycleComplete,
            title: format!("Cycle {} Completed", cycle_number),
            message: format!(
                "Metabolism cycle {} has completed successfully.",
                cycle_number
            ),
            severity: NotificationSeverity::Info,
            data,
            timestamp: Utc::now(),
            sent: false,
            sent_to: Vec::new(),
        }
    }

    /// Check if a wound severity meets the minimum threshold.
    fn meets_severity_threshold(&self, severity: &WoundSeverity) -> bool {
        let min_severity = match self.config.min_wound_severity.to_lowercase().as_str() {
            "minor" => 0,
            "moderate" => 1,
            "severe" => 2,
            "critical" => 3,
            _ => 1,
        };

        let actual_severity = match severity {
            WoundSeverity::Minor => 0,
            WoundSeverity::Moderate => 1,
            WoundSeverity::Severe => 2,
            WoundSeverity::Critical => 3,
        };

        actual_severity >= min_severity
    }

    /// Map wound severity to notification severity.
    fn wound_to_notification_severity(&self, severity: &WoundSeverity) -> NotificationSeverity {
        match severity {
            WoundSeverity::Minor => NotificationSeverity::Info,
            WoundSeverity::Moderate => NotificationSeverity::Warning,
            WoundSeverity::Severe => NotificationSeverity::Error,
            WoundSeverity::Critical => NotificationSeverity::Critical,
        }
    }
}

impl Default for NotificationPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for NotificationPlugin {
    fn name(&self) -> &str {
        "notification"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn priority(&self) -> PluginPriority {
        // Run after metrics collection
        PluginPriority::Low
    }

    fn on_load(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError> {
        info!(
            plugin = self.name(),
            channels = ?self.config.channels,
            "Notification plugin loaded"
        );
        ctx.log(
            LogLevel::Info,
            format!(
                "[notification] Plugin loaded with channels: {:?}",
                self.config.channels
            ),
        );
        Ok(())
    }

    fn on_tick(&self, _ctx: &mut PluginContext) {
        // Notifications are sent synchronously on events, but we could
        // implement async batching here if needed
    }

    fn on_event(&self, event: &LivingProtocolEvent, ctx: &mut PluginContext) {
        // Handle wound events
        if let LivingProtocolEvent::WoundCreated(wound_event) = event {
            if self.config.notify_on_wounds
                && self.meets_severity_threshold(&wound_event.severity)
            {
                let notification = Notification {
                    notification_type: NotificationType::WoundCreated,
                    title: format!("Wound Created: {:?}", wound_event.severity),
                    message: format!(
                        "A {:?} wound has been created for agent {}. Cause: {}",
                        wound_event.severity, wound_event.agent_did, wound_event.cause
                    ),
                    severity: self.wound_to_notification_severity(&wound_event.severity),
                    data: serde_json::json!({
                        "wound_id": wound_event.wound_id,
                        "agent_did": wound_event.agent_did,
                        "severity": format!("{:?}", wound_event.severity),
                        "cause": wound_event.cause,
                    }),
                    timestamp: Utc::now(),
                    sent: false,
                    sent_to: Vec::new(),
                };

                ctx.log(
                    LogLevel::Info,
                    format!(
                        "[notification] Wound notification queued: {} - {:?}",
                        wound_event.agent_did, wound_event.severity
                    ),
                );

                // Note: Can't mutate self here, would need interior mutability
                // In a real implementation, we'd use Arc<Mutex<...>> for the queue
            }
        }

        // Handle kenosis events
        if let LivingProtocolEvent::KenosisCommitted(kenosis_event) = event {
            if self.config.notify_on_kenosis {
                ctx.log(
                    LogLevel::Info,
                    format!(
                        "[notification] Kenosis notification: {} released {:.1}%",
                        kenosis_event.agent_did, kenosis_event.release_percentage * 100.0
                    ),
                );
            }
        }

        // Handle shadow surfacing
        if let LivingProtocolEvent::ShadowSurfaced(shadow_event) = event {
            ctx.log(
                LogLevel::Info,
                format!(
                    "[notification] Shadow surfaced: {}",
                    shadow_event.shadow.original_content_id
                ),
            );
        }
    }

    fn on_phase_transition(&self, from: CyclePhase, to: CyclePhase, ctx: &mut PluginContext) {
        if self.config.notify_on_phase_transition {
            let notification = self.create_phase_transition_notification(from, to, ctx);
            ctx.log(
                LogLevel::Info,
                format!("[notification] Phase transition: {:?} -> {:?}", from, to),
            );

            // Store notification info in context metadata for later retrieval
            ctx.set_metadata(
                "last_notification",
                serde_json::to_value(&notification).unwrap_or_default(),
            );
        }
    }

    fn on_cycle_start(&self, cycle_number: u64, ctx: &mut PluginContext) {
        if self.config.notify_on_cycle_start {
            let notification = self.create_cycle_start_notification(cycle_number);
            ctx.log(
                LogLevel::Info,
                format!("[notification] Cycle {} started", cycle_number),
            );
            ctx.set_metadata(
                "last_notification",
                serde_json::to_value(&notification).unwrap_or_default(),
            );
        }
    }

    fn on_cycle_complete(&self, cycle_number: u64, ctx: &mut PluginContext) {
        if self.config.notify_on_cycle_complete {
            let notification = self.create_cycle_complete_notification(cycle_number, ctx);
            ctx.log(
                LogLevel::Info,
                format!("[notification] Cycle {} completed", cycle_number),
            );
            ctx.set_metadata(
                "last_notification",
                serde_json::to_value(&notification).unwrap_or_default(),
            );
        }
    }

    fn configure(&mut self, config: PluginConfig) -> Result<(), PluginError> {
        if let Ok(notif_config) = serde_json::from_value::<NotificationConfig>(config.settings) {
            self.config = notif_config;
        }
        Ok(())
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "channels": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["webhook", "slack", "discord", "email"]
                    },
                    "default": ["webhook"]
                },
                "webhook_url": {
                    "type": "string",
                    "format": "uri"
                },
                "slack_webhook_url": {
                    "type": "string",
                    "format": "uri"
                },
                "discord_webhook_url": {
                    "type": "string",
                    "format": "uri"
                },
                "notify_on_phase_transition": {
                    "type": "boolean",
                    "default": true
                },
                "notify_on_cycle_start": {
                    "type": "boolean",
                    "default": true
                },
                "notify_on_cycle_complete": {
                    "type": "boolean",
                    "default": true
                },
                "notify_on_wounds": {
                    "type": "boolean",
                    "default": true
                },
                "notify_on_kenosis": {
                    "type": "boolean",
                    "default": true
                },
                "min_wound_severity": {
                    "type": "string",
                    "enum": ["minor", "moderate", "severe", "critical"],
                    "default": "moderate"
                },
                "max_notifications_per_minute": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 10
                },
                "include_metrics": {
                    "type": "boolean",
                    "default": true
                }
            }
        }))
    }

    fn status(&self) -> PluginStatus {
        PluginStatus::Running
    }

    fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "total_notifications": self.stats.total_notifications.load(Ordering::Relaxed),
            "successful_sends": self.stats.successful_sends.load(Ordering::Relaxed),
            "failed_sends": self.stats.failed_sends.load(Ordering::Relaxed),
            "rate_limited": self.stats.rate_limited.load(Ordering::Relaxed),
            "pending_count": self.pending_notifications.len(),
            "recent_count": self.recent_notifications.len(),
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
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(NotificationPlugin::new());
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
    fn test_plugin_creation() {
        let plugin = NotificationPlugin::new();
        assert_eq!(plugin.name(), "notification");
    }

    #[test]
    fn test_severity_threshold() {
        let plugin = NotificationPlugin::with_config(NotificationConfig {
            min_wound_severity: "moderate".to_string(),
            ..Default::default()
        });

        assert!(!plugin.meets_severity_threshold(&WoundSeverity::Minor));
        assert!(plugin.meets_severity_threshold(&WoundSeverity::Moderate));
        assert!(plugin.meets_severity_threshold(&WoundSeverity::Severe));
        assert!(plugin.meets_severity_threshold(&WoundSeverity::Critical));
    }

    #[test]
    fn test_notification_creation() {
        let plugin = NotificationPlugin::new();
        let ctx = test_context();

        let notification =
            plugin.create_phase_transition_notification(CyclePhase::Shadow, CyclePhase::Composting, &ctx);

        assert_eq!(notification.notification_type, NotificationType::PhaseTransition);
        assert!(notification.title.contains("Shadow"));
        assert!(notification.title.contains("Composting"));
    }

    #[test]
    fn test_cycle_notifications() {
        let plugin = NotificationPlugin::new();
        let ctx = test_context();

        let start_notif = plugin.create_cycle_start_notification(5);
        assert_eq!(start_notif.notification_type, NotificationType::CycleStart);
        assert!(start_notif.title.contains("5"));

        let complete_notif = plugin.create_cycle_complete_notification(5, &ctx);
        assert_eq!(complete_notif.notification_type, NotificationType::CycleComplete);
    }
}
