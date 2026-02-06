//! Webhook delivery system for the Living Protocol.
//!
//! Allows external systems to receive notifications of protocol events
//! via HTTP POST requests with HMAC-SHA256 signatures for verification.
//!
//! ## Security
//!
//! All webhook payloads are signed using HMAC-SHA256. The signature is
//! included in the `X-Webhook-Signature` header as a hex-encoded string.
//!
//! To verify the signature:
//! 1. Compute HMAC-SHA256 of the raw request body using your webhook secret
//! 2. Hex-encode the result
//! 3. Compare with the `X-Webhook-Signature` header value
//!
//! ## Retry Policy
//!
//! Failed webhook deliveries are retried with exponential backoff:
//! - Initial delay: 1 second
//! - Maximum delay: 5 minutes
//! - Maximum attempts: 5

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use living_core::LivingProtocolEvent;

/// HMAC-SHA256 type alias.
type HmacSha256 = Hmac<Sha256>;

/// Configuration for a single webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Unique identifier for this webhook
    pub id: String,
    /// URL to POST events to
    pub url: String,
    /// Secret key for HMAC-SHA256 signing
    pub secret: String,
    /// Event types to subscribe to (empty = all events)
    pub events: HashSet<String>,
    /// Whether the webhook is enabled
    pub enabled: bool,
    /// Optional description
    pub description: Option<String>,
    /// Custom headers to include
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

impl WebhookConfig {
    /// Create a new webhook configuration.
    pub fn new(url: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            url: url.into(),
            secret: secret.into(),
            events: HashSet::new(),
            enabled: true,
            description: None,
            headers: HashMap::new(),
        }
    }

    /// Set the events to subscribe to.
    pub fn with_events(mut self, events: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.events = events.into_iter().map(Into::into).collect();
        self
    }

    /// Set a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a custom header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Check if this webhook should receive the given event type.
    pub fn should_receive(&self, event_type: &str) -> bool {
        if !self.enabled {
            return false;
        }
        if self.events.is_empty() {
            return true; // Subscribe to all events
        }
        self.events
            .iter()
            .any(|e| e.eq_ignore_ascii_case(event_type))
    }
}

/// Webhook delivery payload.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    /// Unique delivery ID
    pub delivery_id: String,
    /// Event type
    pub event_type: String,
    /// Event data
    pub event: serde_json::Value,
    /// ISO8601 timestamp
    pub timestamp: String,
    /// Webhook ID this is being delivered to
    pub webhook_id: String,
}

/// Delivery attempt result.
#[derive(Debug, Clone)]
pub struct DeliveryResult {
    pub webhook_id: String,
    pub delivery_id: String,
    pub success: bool,
    pub status_code: Option<u16>,
    pub error: Option<String>,
    pub attempts: u32,
    pub duration_ms: u64,
}

/// Retry configuration.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Maximum number of attempts
    pub max_attempts: u32,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(300), // 5 minutes
            max_attempts: 5,
            backoff_multiplier: 2.0,
        }
    }
}

/// Webhook manager that handles event dispatching.
pub struct WebhookManager {
    /// Registered webhooks
    webhooks: Arc<RwLock<Vec<WebhookConfig>>>,
    /// HTTP client for making requests
    client: Client,
    /// Retry configuration
    retry_config: RetryConfig,
    /// Delivery results channel
    results_tx: broadcast::Sender<DeliveryResult>,
}

impl WebhookManager {
    /// Create a new webhook manager.
    pub fn new() -> Self {
        let (results_tx, _) = broadcast::channel(1024);

        Self {
            webhooks: Arc::new(RwLock::new(Vec::new())),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("Mycelix-Webhook/0.6.0")
                .build()
                .expect("Failed to create HTTP client"),
            retry_config: RetryConfig::default(),
            results_tx,
        }
    }

    /// Create with custom retry configuration.
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Register a webhook.
    pub async fn register(&self, config: WebhookConfig) {
        let mut webhooks = self.webhooks.write().await;
        info!(
            webhook_id = %config.id,
            url = %config.url,
            events = ?config.events,
            "Registering webhook"
        );
        webhooks.push(config);
    }

    /// Unregister a webhook by ID.
    pub async fn unregister(&self, webhook_id: &str) -> bool {
        let mut webhooks = self.webhooks.write().await;
        let len_before = webhooks.len();
        webhooks.retain(|w| w.id != webhook_id);
        let removed = webhooks.len() < len_before;
        if removed {
            info!(webhook_id = %webhook_id, "Unregistered webhook");
        }
        removed
    }

    /// Get all registered webhooks.
    pub async fn list_webhooks(&self) -> Vec<WebhookConfig> {
        self.webhooks.read().await.clone()
    }

    /// Subscribe to delivery results.
    pub fn subscribe_results(&self) -> broadcast::Receiver<DeliveryResult> {
        self.results_tx.subscribe()
    }

    /// Dispatch an event to all matching webhooks.
    pub async fn dispatch(&self, event: &LivingProtocolEvent) {
        let event_type = get_event_type(event);
        let event_json = match serde_json::to_value(event) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize event: {}", e);
                return;
            }
        };

        let webhooks = self.webhooks.read().await;
        let matching: Vec<_> = webhooks
            .iter()
            .filter(|w| w.should_receive(event_type))
            .cloned()
            .collect();
        drop(webhooks);

        if matching.is_empty() {
            debug!(event_type = %event_type, "No webhooks registered for event");
            return;
        }

        debug!(
            event_type = %event_type,
            webhook_count = matching.len(),
            "Dispatching event to webhooks"
        );

        // Dispatch to each webhook concurrently
        let futures: Vec<_> = matching
            .into_iter()
            .map(|webhook| {
                let client = self.client.clone();
                let retry_config = self.retry_config.clone();
                let results_tx = self.results_tx.clone();
                let event_json = event_json.clone();
                let event_type = event_type.to_string();

                tokio::spawn(async move {
                    let result = deliver_with_retry(
                        &client,
                        &webhook,
                        &event_type,
                        &event_json,
                        &retry_config,
                    )
                    .await;

                    // Send result to subscribers
                    let _ = results_tx.send(result);
                })
            })
            .collect();

        // Wait for all deliveries to complete
        for future in futures {
            let _ = future.await;
        }
    }

    /// Start listening for events and dispatching webhooks.
    pub async fn start_dispatcher(
        self: Arc<Self>,
        mut event_rx: broadcast::Receiver<String>,
    ) {
        info!("Webhook dispatcher started");

        loop {
            match event_rx.recv().await {
                Ok(json) => {
                    match serde_json::from_str::<LivingProtocolEvent>(&json) {
                        Ok(event) => {
                            self.dispatch(&event).await;
                        }
                        Err(e) => {
                            warn!("Failed to parse event for webhook dispatch: {}", e);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Webhook dispatcher lagged behind by {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Event channel closed, stopping webhook dispatcher");
                    break;
                }
            }
        }
    }
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute HMAC-SHA256 signature for a payload.
fn compute_signature(secret: &str, payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Deliver a webhook with retry logic.
async fn deliver_with_retry(
    client: &Client,
    webhook: &WebhookConfig,
    event_type: &str,
    event_json: &serde_json::Value,
    retry_config: &RetryConfig,
) -> DeliveryResult {
    let delivery_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let payload = WebhookPayload {
        delivery_id: delivery_id.clone(),
        event_type: event_type.to_string(),
        event: event_json.clone(),
        timestamp,
        webhook_id: webhook.id.clone(),
    };

    let body = match serde_json::to_string(&payload) {
        Ok(b) => b,
        Err(e) => {
            return DeliveryResult {
                webhook_id: webhook.id.clone(),
                delivery_id,
                success: false,
                status_code: None,
                error: Some(format!("Failed to serialize payload: {}", e)),
                attempts: 0,
                duration_ms: 0,
            };
        }
    };

    let signature = compute_signature(&webhook.secret, body.as_bytes());

    let mut attempts = 0;
    let mut delay = retry_config.initial_delay;
    let start = std::time::Instant::now();

    loop {
        attempts += 1;

        let mut request = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &signature)
            .header("X-Webhook-ID", &webhook.id)
            .header("X-Delivery-ID", &delivery_id);

        // Add custom headers
        for (key, value) in &webhook.headers {
            request = request.header(key, value);
        }

        match request.body(body.clone()).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    debug!(
                        webhook_id = %webhook.id,
                        delivery_id = %delivery_id,
                        status = %status,
                        attempts = attempts,
                        "Webhook delivered successfully"
                    );
                    return DeliveryResult {
                        webhook_id: webhook.id.clone(),
                        delivery_id,
                        success: true,
                        status_code: Some(status.as_u16()),
                        error: None,
                        attempts,
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }

                // Non-2xx response
                if attempts >= retry_config.max_attempts {
                    warn!(
                        webhook_id = %webhook.id,
                        delivery_id = %delivery_id,
                        status = %status,
                        attempts = attempts,
                        "Webhook delivery failed after max attempts"
                    );
                    return DeliveryResult {
                        webhook_id: webhook.id.clone(),
                        delivery_id,
                        success: false,
                        status_code: Some(status.as_u16()),
                        error: Some(format!("HTTP {}", status)),
                        attempts,
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }

                debug!(
                    webhook_id = %webhook.id,
                    delivery_id = %delivery_id,
                    status = %status,
                    attempt = attempts,
                    retry_delay_ms = delay.as_millis(),
                    "Webhook delivery failed, retrying"
                );
            }
            Err(e) => {
                if attempts >= retry_config.max_attempts {
                    error!(
                        webhook_id = %webhook.id,
                        delivery_id = %delivery_id,
                        error = %e,
                        attempts = attempts,
                        "Webhook delivery error after max attempts"
                    );
                    return DeliveryResult {
                        webhook_id: webhook.id.clone(),
                        delivery_id,
                        success: false,
                        status_code: None,
                        error: Some(e.to_string()),
                        attempts,
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }

                debug!(
                    webhook_id = %webhook.id,
                    delivery_id = %delivery_id,
                    error = %e,
                    attempt = attempts,
                    retry_delay_ms = delay.as_millis(),
                    "Webhook delivery error, retrying"
                );
            }
        }

        // Wait before retrying
        tokio::time::sleep(delay).await;

        // Exponential backoff
        delay = Duration::from_secs_f64(
            (delay.as_secs_f64() * retry_config.backoff_multiplier)
                .min(retry_config.max_delay.as_secs_f64()),
        );
    }
}

/// Get the event type name from an event.
fn get_event_type(event: &LivingProtocolEvent) -> &'static str {
    match event {
        LivingProtocolEvent::CompostingStarted(_) => "CompostingStarted",
        LivingProtocolEvent::NutrientExtracted(_) => "NutrientExtracted",
        LivingProtocolEvent::CompostingCompleted(_) => "CompostingCompleted",
        LivingProtocolEvent::WoundCreated(_) => "WoundCreated",
        LivingProtocolEvent::WoundPhaseAdvanced(_) => "WoundPhaseAdvanced",
        LivingProtocolEvent::RestitutionFulfilled(_) => "RestitutionFulfilled",
        LivingProtocolEvent::ScarTissueFormed(_) => "ScarTissueFormed",
        LivingProtocolEvent::MetabolicTrustUpdated(_) => "MetabolicTrustUpdated",
        LivingProtocolEvent::KenosisCommitted(_) => "KenosisCommitted",
        LivingProtocolEvent::KenosisExecuted(_) => "KenosisExecuted",
        LivingProtocolEvent::TemporalKVectorUpdated(_) => "TemporalKVectorUpdated",
        LivingProtocolEvent::FieldInterferenceDetected(_) => "FieldInterferenceDetected",
        LivingProtocolEvent::DreamStateChanged(_) => "DreamStateChanged",
        LivingProtocolEvent::DreamProposalGenerated(_) => "DreamProposalGenerated",
        LivingProtocolEvent::NetworkPhiComputed(_) => "NetworkPhiComputed",
        LivingProtocolEvent::ShadowSurfaced(_) => "ShadowSurfaced",
        LivingProtocolEvent::ClaimHeldInUncertainty(_) => "ClaimHeldInUncertainty",
        LivingProtocolEvent::ClaimReleasedFromUncertainty(_) => "ClaimReleasedFromUncertainty",
        LivingProtocolEvent::SilenceDetected(_) => "SilenceDetected",
        LivingProtocolEvent::BeautyScored(_) => "BeautyScored",
        LivingProtocolEvent::EntanglementFormed(_) => "EntanglementFormed",
        LivingProtocolEvent::EntanglementDecayed(_) => "EntanglementDecayed",
        LivingProtocolEvent::AttractorFieldComputed(_) => "AttractorFieldComputed",
        LivingProtocolEvent::LiminalTransitionStarted(_) => "LiminalTransitionStarted",
        LivingProtocolEvent::LiminalTransitionCompleted(_) => "LiminalTransitionCompleted",
        LivingProtocolEvent::InterSpeciesRegistered(_) => "InterSpeciesRegistered",
        LivingProtocolEvent::ResonanceAddressCreated(_) => "ResonanceAddressCreated",
        LivingProtocolEvent::FractalPatternReplicated(_) => "FractalPatternReplicated",
        LivingProtocolEvent::MorphogeneticFieldUpdated(_) => "MorphogeneticFieldUpdated",
        LivingProtocolEvent::TimeCrystalPeriodStarted(_) => "TimeCrystalPeriodStarted",
        LivingProtocolEvent::MycelialTaskDistributed(_) => "MycelialTaskDistributed",
        LivingProtocolEvent::MycelialTaskCompleted(_) => "MycelialTaskCompleted",
        LivingProtocolEvent::PhaseTransitioned(_) => "PhaseTransitioned",
        LivingProtocolEvent::CycleStarted(_) => "CycleStarted",
    }
}

/// Parse webhook events from a comma-separated string.
pub fn parse_webhook_events(events_str: &str) -> HashSet<String> {
    events_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_new() {
        let config = WebhookConfig::new("https://example.com/webhook", "secret123");
        assert!(!config.id.is_empty());
        assert_eq!(config.url, "https://example.com/webhook");
        assert_eq!(config.secret, "secret123");
        assert!(config.enabled);
        assert!(config.events.is_empty());
    }

    #[test]
    fn test_webhook_config_with_events() {
        let config = WebhookConfig::new("https://example.com/webhook", "secret")
            .with_events(["PhaseTransitioned", "CycleStarted"]);

        assert!(config.events.contains("PhaseTransitioned"));
        assert!(config.events.contains("CycleStarted"));
    }

    #[test]
    fn test_should_receive_all_events() {
        let config = WebhookConfig::new("https://example.com/webhook", "secret");
        assert!(config.should_receive("PhaseTransitioned"));
        assert!(config.should_receive("CycleStarted"));
        assert!(config.should_receive("AnyEvent"));
    }

    #[test]
    fn test_should_receive_filtered_events() {
        let config = WebhookConfig::new("https://example.com/webhook", "secret")
            .with_events(["PhaseTransitioned", "CycleStarted"]);

        assert!(config.should_receive("PhaseTransitioned"));
        assert!(config.should_receive("phasetransitioned")); // Case insensitive
        assert!(config.should_receive("CycleStarted"));
        assert!(!config.should_receive("WoundCreated"));
    }

    #[test]
    fn test_should_receive_disabled() {
        let mut config = WebhookConfig::new("https://example.com/webhook", "secret");
        config.enabled = false;
        assert!(!config.should_receive("PhaseTransitioned"));
    }

    #[test]
    fn test_compute_signature() {
        let secret = "test-secret";
        let payload = b"test payload";
        let signature = compute_signature(secret, payload);

        // Verify it's a valid hex string
        assert!(!signature.is_empty());
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(signature.len(), 64); // SHA256 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_compute_signature_deterministic() {
        let secret = "test-secret";
        let payload = b"test payload";

        let sig1 = compute_signature(secret, payload);
        let sig2 = compute_signature(secret, payload);

        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_compute_signature_different_secrets() {
        let payload = b"test payload";

        let sig1 = compute_signature("secret1", payload);
        let sig2 = compute_signature("secret2", payload);

        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_parse_webhook_events() {
        let events = parse_webhook_events("PhaseTransitioned, CycleStarted, WoundCreated");
        assert_eq!(events.len(), 3);
        assert!(events.contains("PhaseTransitioned"));
        assert!(events.contains("CycleStarted"));
        assert!(events.contains("WoundCreated"));
    }

    #[test]
    fn test_parse_webhook_events_empty() {
        let events = parse_webhook_events("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.initial_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(300));
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[tokio::test]
    async fn test_webhook_manager_register_unregister() {
        let manager = WebhookManager::new();

        let config = WebhookConfig::new("https://example.com/webhook", "secret");
        let webhook_id = config.id.clone();

        manager.register(config).await;

        let webhooks = manager.list_webhooks().await;
        assert_eq!(webhooks.len(), 1);
        assert_eq!(webhooks[0].id, webhook_id);

        assert!(manager.unregister(&webhook_id).await);

        let webhooks = manager.list_webhooks().await;
        assert!(webhooks.is_empty());
    }

    #[test]
    fn test_get_event_type() {
        let event = LivingProtocolEvent::CycleStarted(living_core::CycleStartedEvent {
            cycle_number: 1,
            started_at: chrono::Utc::now(),
        });
        assert_eq!(get_event_type(&event), "CycleStarted");
    }
}
