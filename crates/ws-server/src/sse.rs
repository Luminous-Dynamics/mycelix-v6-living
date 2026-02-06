//! Server-Sent Events (SSE) endpoint for the Living Protocol.
//!
//! Provides a streaming HTTP endpoint for clients that cannot use WebSocket
//! or GraphQL subscriptions.
//!
//! ## Endpoint
//!
//! `GET /api/v1/events`
//!
//! ## Query Parameters
//!
//! - `event_types` - Comma-separated list of event types to subscribe to
//! - `phases` - Comma-separated list of phases to filter by (for phase transitions)
//!
//! ## Example
//!
//! ```bash
//! # Subscribe to all events
//! curl -N "http://localhost:8892/api/v1/events"
//!
//! # Subscribe to specific event types
//! curl -N "http://localhost:8892/api/v1/events?event_types=PhaseTransitioned,CycleStarted"
//!
//! # Filter phase transitions by specific phases
//! curl -N "http://localhost:8892/api/v1/events?phases=Shadow,Composting"
//! ```

use std::collections::HashSet;
use std::convert::Infallible;

use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures_util::stream::{self, Stream, StreamExt as FuturesStreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, info, warn};

use living_core::{CyclePhase, LivingProtocolEvent};

/// Configuration for the SSE server.
#[derive(Debug, Clone)]
pub struct SseConfig {
    /// Port to listen on
    pub port: u16,
    /// Host to bind to
    pub host: String,
    /// Keep-alive interval in seconds
    pub keep_alive_seconds: u64,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            port: 8892,
            host: "127.0.0.1".to_string(),
            keep_alive_seconds: 30,
        }
    }
}

/// Query parameters for the events endpoint.
#[derive(Debug, Default, Deserialize)]
pub struct EventsQuery {
    /// Comma-separated list of event types to filter by
    pub event_types: Option<String>,
    /// Comma-separated list of phases to filter by
    pub phases: Option<String>,
}

impl EventsQuery {
    /// Parse event types filter into a set.
    pub fn event_type_filter(&self) -> Option<HashSet<String>> {
        self.event_types.as_ref().map(|types| {
            types
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
    }

    /// Parse phases filter into a set.
    pub fn phase_filter(&self) -> Option<HashSet<String>> {
        self.phases.as_ref().map(|phases| {
            phases
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
    }
}

/// SSE event data wrapper.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SseEventData {
    /// Event type name
    pub event_type: String,
    /// Event payload
    pub data: serde_json::Value,
    /// Server timestamp
    pub timestamp: String,
}

/// State for the SSE router.
#[derive(Clone)]
pub struct SseState {
    pub event_tx: broadcast::Sender<String>,
    pub keep_alive_seconds: u64,
}

/// Get the event type name from a LivingProtocolEvent.
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

/// Get the phase from a phase transition event if applicable.
fn get_transition_phases(event: &LivingProtocolEvent) -> Option<(CyclePhase, CyclePhase)> {
    if let LivingProtocolEvent::PhaseTransitioned(ref e) = event {
        Some((e.transition.from, e.transition.to))
    } else {
        None
    }
}

/// Check if an event should pass the filter.
fn should_include_event(
    event: &LivingProtocolEvent,
    event_type_filter: &Option<HashSet<String>>,
    phase_filter: &Option<HashSet<String>>,
) -> bool {
    let event_type = get_event_type(event);

    // Check event type filter
    if let Some(ref types) = event_type_filter {
        if !types.iter().any(|t| t.eq_ignore_ascii_case(event_type)) {
            return false;
        }
    }

    // Check phase filter (only applies to PhaseTransitioned events)
    if let Some(ref phases) = phase_filter {
        if let Some((from, to)) = get_transition_phases(event) {
            let from_name = format!("{:?}", from);
            let to_name = format!("{:?}", to);

            // Include if either from or to phase matches the filter
            let matches = phases
                .iter()
                .any(|p| p.eq_ignore_ascii_case(&from_name) || p.eq_ignore_ascii_case(&to_name));

            if !matches {
                return false;
            }
        }
    }

    true
}

/// SSE events endpoint handler.
async fn events_handler(
    State(state): State<SseState>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let event_rx = state.event_tx.subscribe();
    let event_type_filter = query.event_type_filter();
    let phase_filter = query.phase_filter();

    debug!(
        event_types = ?event_type_filter,
        phases = ?phase_filter,
        "New SSE connection"
    );

    // Transform broadcast stream into SSE events
    // BroadcastStream yields Result<String, BroadcastStreamRecvError>
    let stream = BroadcastStream::new(event_rx)
        .filter_map(move |result| {
            let event_type_filter = event_type_filter.clone();
            let phase_filter = phase_filter.clone();

            async move {
                match result {
                    Ok(json) => {
                        // Parse the event
                        match serde_json::from_str::<LivingProtocolEvent>(&json) {
                            Ok(event) => {
                                // Apply filters
                                if !should_include_event(&event, &event_type_filter, &phase_filter) {
                                    return None;
                                }

                                let event_type = get_event_type(&event);
                                let data = SseEventData {
                                    event_type: event_type.to_string(),
                                    data: serde_json::from_str(&json).unwrap_or(serde_json::Value::Null),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                };

                                match serde_json::to_string(&data) {
                                    Ok(json_data) => {
                                        Some(Ok::<Event, Infallible>(Event::default().event(event_type).data(json_data)))
                                    }
                                    Err(e) => {
                                        warn!("Failed to serialize SSE event: {}", e);
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse event JSON: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Broadcast receive error: {}", e);
                        None
                    }
                }
            }
        });

    // Add a heartbeat comment to keep the connection alive
    let heartbeat = stream::iter(vec![Ok(Event::default().comment("connected"))]);
    let stream = heartbeat.chain(stream);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(state.keep_alive_seconds))
            .text("keep-alive"),
    )
}

/// Health check endpoint.
async fn health_handler() -> &'static str {
    "OK"
}

/// Create the SSE router.
pub fn create_sse_router(event_tx: broadcast::Sender<String>, keep_alive_seconds: u64) -> Router {
    let state = SseState {
        event_tx,
        keep_alive_seconds,
    };

    Router::new()
        .route("/api/v1/events", get(events_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}

/// Run the SSE server.
pub async fn run_sse_server(
    config: SseConfig,
    event_tx: broadcast::Sender<String>,
) -> anyhow::Result<()> {
    let app = create_sse_router(event_tx, config.keep_alive_seconds);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!(address = %addr, "SSE server listening");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_query_parsing() {
        let query = EventsQuery {
            event_types: Some("PhaseTransitioned, CycleStarted".to_string()),
            phases: Some("Shadow,Composting".to_string()),
        };

        let types = query.event_type_filter().unwrap();
        assert!(types.contains("PhaseTransitioned"));
        assert!(types.contains("CycleStarted"));

        let phases = query.phase_filter().unwrap();
        assert!(phases.contains("Shadow"));
        assert!(phases.contains("Composting"));
    }

    #[test]
    fn test_events_query_empty() {
        let query = EventsQuery {
            event_types: None,
            phases: None,
        };

        assert!(query.event_type_filter().is_none());
        assert!(query.phase_filter().is_none());
    }

    #[test]
    fn test_event_type_mapping() {
        let event = LivingProtocolEvent::CycleStarted(living_core::CycleStartedEvent {
            cycle_number: 1,
            started_at: chrono::Utc::now(),
        });
        assert_eq!(get_event_type(&event), "CycleStarted");
    }

    #[test]
    fn test_should_include_event_no_filter() {
        let event = LivingProtocolEvent::CycleStarted(living_core::CycleStartedEvent {
            cycle_number: 1,
            started_at: chrono::Utc::now(),
        });

        assert!(should_include_event(&event, &None, &None));
    }

    #[test]
    fn test_should_include_event_with_type_filter() {
        let event = LivingProtocolEvent::CycleStarted(living_core::CycleStartedEvent {
            cycle_number: 1,
            started_at: chrono::Utc::now(),
        });

        let mut filter = HashSet::new();
        filter.insert("CycleStarted".to_string());

        assert!(should_include_event(&event, &Some(filter.clone()), &None));

        let mut wrong_filter = HashSet::new();
        wrong_filter.insert("PhaseTransitioned".to_string());

        assert!(!should_include_event(&event, &Some(wrong_filter), &None));
    }

    #[test]
    fn test_sse_config_default() {
        let config = SseConfig::default();
        assert_eq!(config.port, 8892);
        assert_eq!(config.keep_alive_seconds, 30);
    }
}
