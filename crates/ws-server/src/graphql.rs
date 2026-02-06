//! GraphQL API for the Living Protocol.
//!
//! Provides a GraphQL interface for querying cycle state and subscribing to events.
//!
//! ## Schema Overview
//!
//! ### Queries
//! - `cycleState` - Current state of the metabolism cycle
//! - `currentPhase` - Current phase name
//! - `cycleNumber` - Current cycle number
//! - `transitionHistory` - History of phase transitions
//! - `phaseMetrics(phase: Phase!)` - Metrics for a specific phase
//!
//! ### Subscriptions
//! - `onPhaseChange` - Notifies when the phase changes
//! - `onCycleStart` - Notifies when a new cycle starts
//! - `onEvent` - Stream all Living Protocol events

use std::sync::Arc;

use async_graphql::{
    Context, EmptyMutation, Enum, InputObject, Object, Schema, SimpleObject, Subscription,
};
use futures_util::Stream;
use tokio::sync::{broadcast, RwLock};
use tracing::info;

use cycle_engine::MetabolismCycleEngine;
use living_core::{CyclePhase, LivingProtocolEvent};

/// GraphQL Phase enum mapping to CyclePhase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum Phase {
    Shadow,
    Composting,
    Liminal,
    NegativeCapability,
    Eros,
    CoCreation,
    Beauty,
    EmergentPersonhood,
    Kenosis,
}

impl From<CyclePhase> for Phase {
    fn from(phase: CyclePhase) -> Self {
        match phase {
            CyclePhase::Shadow => Self::Shadow,
            CyclePhase::Composting => Self::Composting,
            CyclePhase::Liminal => Self::Liminal,
            CyclePhase::NegativeCapability => Self::NegativeCapability,
            CyclePhase::Eros => Self::Eros,
            CyclePhase::CoCreation => Self::CoCreation,
            CyclePhase::Beauty => Self::Beauty,
            CyclePhase::EmergentPersonhood => Self::EmergentPersonhood,
            CyclePhase::Kenosis => Self::Kenosis,
        }
    }
}

impl From<Phase> for CyclePhase {
    fn from(phase: Phase) -> Self {
        match phase {
            Phase::Shadow => Self::Shadow,
            Phase::Composting => Self::Composting,
            Phase::Liminal => Self::Liminal,
            Phase::NegativeCapability => Self::NegativeCapability,
            Phase::Eros => Self::Eros,
            Phase::CoCreation => Self::CoCreation,
            Phase::Beauty => Self::Beauty,
            Phase::EmergentPersonhood => Self::EmergentPersonhood,
            Phase::Kenosis => Self::Kenosis,
        }
    }
}

/// Cycle state representation for GraphQL.
#[derive(Debug, Clone, SimpleObject)]
pub struct CycleState {
    /// Current cycle number (starts at 1)
    pub cycle_number: u64,
    /// Current phase of the cycle
    pub current_phase: Phase,
    /// ISO8601 timestamp when the current phase started
    pub phase_started: String,
    /// ISO8601 timestamp when the current cycle started
    pub cycle_started: String,
    /// Day within the current phase (0-indexed)
    pub phase_day: u32,
    /// Duration of the current phase in days
    pub phase_duration_days: u32,
    /// Whether the engine is currently running
    pub is_running: bool,
}

/// Phase transition record for GraphQL.
#[derive(Debug, Clone, SimpleObject)]
pub struct PhaseTransition {
    /// Phase transitioned from
    pub from_phase: Phase,
    /// Phase transitioned to
    pub to_phase: Phase,
    /// Cycle number when transition occurred
    pub cycle_number: u64,
    /// ISO8601 timestamp of the transition
    pub transitioned_at: String,
}

/// Phase metrics for GraphQL.
#[derive(Debug, Clone, SimpleObject)]
pub struct PhaseMetrics {
    /// Phase these metrics are for
    pub phase: Phase,
    /// Number of active agents
    pub active_agents: u64,
    /// Spectral K value
    pub spectral_k: f64,
    /// Mean metabolic trust across agents
    pub mean_metabolic_trust: f64,
    /// Number of active wounds being healed
    pub active_wounds: u64,
    /// Number of entities being composted
    pub composting_entities: u64,
    /// Number of entities in liminal transition
    pub liminal_entities: u64,
    /// Number of entangled agent pairs
    pub entangled_pairs: u64,
    /// Number of claims held in uncertainty
    pub held_uncertainties: u64,
}

/// Phase change event for subscriptions.
#[derive(Debug, Clone, SimpleObject)]
pub struct PhaseChangeEvent {
    /// Previous phase
    pub from_phase: Phase,
    /// New phase
    pub to_phase: Phase,
    /// Cycle number
    pub cycle_number: u64,
    /// ISO8601 timestamp
    pub timestamp: String,
}

/// Cycle start event for subscriptions.
#[derive(Debug, Clone, SimpleObject)]
pub struct CycleStartEvent {
    /// New cycle number
    pub cycle_number: u64,
    /// ISO8601 timestamp
    pub started_at: String,
}

/// Generic event wrapper for the onEvent subscription.
#[derive(Debug, Clone, SimpleObject)]
pub struct ProtocolEvent {
    /// Event type name
    pub event_type: String,
    /// Event payload as JSON string
    pub payload: String,
    /// ISO8601 timestamp
    pub timestamp: String,
}

/// Input for filtering phase metrics query.
#[derive(Debug, Clone, InputObject)]
pub struct PhaseInput {
    pub phase: Phase,
}

/// GraphQL Query root.
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get the current cycle state.
    async fn cycle_state(&self, ctx: &Context<'_>) -> async_graphql::Result<CycleState> {
        let engine = ctx.data::<Arc<RwLock<MetabolismCycleEngine>>>()?;
        let engine = engine.read().await;

        Ok(CycleState {
            cycle_number: engine.cycle_number(),
            current_phase: engine.current_phase().into(),
            phase_started: engine.phase_started().to_rfc3339(),
            cycle_started: engine.cycle_started().to_rfc3339(),
            phase_day: engine.phase_day(),
            phase_duration_days: engine.current_phase().duration_days(),
            is_running: engine.is_running(),
        })
    }

    /// Get the current phase.
    async fn current_phase(&self, ctx: &Context<'_>) -> async_graphql::Result<Phase> {
        let engine = ctx.data::<Arc<RwLock<MetabolismCycleEngine>>>()?;
        let engine = engine.read().await;
        Ok(engine.current_phase().into())
    }

    /// Get the current cycle number.
    async fn cycle_number(&self, ctx: &Context<'_>) -> async_graphql::Result<u64> {
        let engine = ctx.data::<Arc<RwLock<MetabolismCycleEngine>>>()?;
        let engine = engine.read().await;
        Ok(engine.cycle_number())
    }

    /// Get the transition history.
    /// The `limit` parameter specifies the maximum number of transitions to return (default: all).
    async fn transition_history(
        &self,
        ctx: &Context<'_>,
        limit: Option<usize>,
    ) -> async_graphql::Result<Vec<PhaseTransition>> {
        let engine = ctx.data::<Arc<RwLock<MetabolismCycleEngine>>>()?;
        let engine = engine.read().await;

        let history = engine.transition_history();
        let transitions: Vec<PhaseTransition> = history
            .iter()
            .rev() // Most recent first
            .take(limit.unwrap_or(usize::MAX))
            .map(|t| PhaseTransition {
                from_phase: t.from.into(),
                to_phase: t.to.into(),
                cycle_number: t.cycle_number,
                transitioned_at: t.transitioned_at.to_rfc3339(),
            })
            .collect();

        Ok(transitions)
    }

    /// Get metrics for a specific phase.
    async fn phase_metrics(
        &self,
        ctx: &Context<'_>,
        phase: Phase,
    ) -> async_graphql::Result<PhaseMetrics> {
        let engine = ctx.data::<Arc<RwLock<MetabolismCycleEngine>>>()?;
        let engine = engine.read().await;

        let metrics = engine.phase_metrics(phase.into());

        Ok(PhaseMetrics {
            phase,
            active_agents: metrics.active_agents,
            spectral_k: metrics.spectral_k,
            mean_metabolic_trust: metrics.mean_metabolic_trust,
            active_wounds: metrics.active_wounds,
            composting_entities: metrics.composting_entities,
            liminal_entities: metrics.liminal_entities,
            entangled_pairs: metrics.entangled_pairs,
            held_uncertainties: metrics.held_uncertainties,
        })
    }

    /// Check if an operation is permitted in the current phase.
    async fn is_operation_permitted(
        &self,
        ctx: &Context<'_>,
        operation: String,
    ) -> async_graphql::Result<bool> {
        let engine = ctx.data::<Arc<RwLock<MetabolismCycleEngine>>>()?;
        let engine = engine.read().await;
        Ok(engine.is_operation_permitted(&operation))
    }
}

/// GraphQL Subscription root.
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to phase change events.
    async fn on_phase_change(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<impl Stream<Item = PhaseChangeEvent>> {
        let event_rx = ctx.data::<broadcast::Sender<String>>()?.subscribe();

        Ok(async_stream::stream! {
            let mut rx = event_rx;
            while let Ok(json) = rx.recv().await {
                if let Ok(event) = serde_json::from_str::<LivingProtocolEvent>(&json) {
                    if let LivingProtocolEvent::PhaseTransitioned(ref transition_event) = event {
                        let transition = &transition_event.transition;
                        yield PhaseChangeEvent {
                            from_phase: transition.from.into(),
                            to_phase: transition.to.into(),
                            cycle_number: transition.cycle_number,
                            timestamp: transition_event.timestamp.to_rfc3339(),
                        };
                    }
                }
            }
        })
    }

    /// Subscribe to cycle start events.
    async fn on_cycle_start(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<impl Stream<Item = CycleStartEvent>> {
        let event_rx = ctx.data::<broadcast::Sender<String>>()?.subscribe();

        Ok(async_stream::stream! {
            let mut rx = event_rx;
            while let Ok(json) = rx.recv().await {
                if let Ok(event) = serde_json::from_str::<LivingProtocolEvent>(&json) {
                    if let LivingProtocolEvent::CycleStarted(ref cycle_event) = event {
                        yield CycleStartEvent {
                            cycle_number: cycle_event.cycle_number,
                            started_at: cycle_event.started_at.to_rfc3339(),
                        };
                    }
                }
            }
        })
    }

    /// Subscribe to all Living Protocol events.
    /// The `event_type` parameter optionally filters by event type name.
    async fn on_event(
        &self,
        ctx: &Context<'_>,
        event_type: Option<String>,
    ) -> async_graphql::Result<impl Stream<Item = ProtocolEvent>> {
        let event_rx = ctx.data::<broadcast::Sender<String>>()?.subscribe();

        Ok(async_stream::stream! {
            let mut rx = event_rx;
            while let Ok(json) = rx.recv().await {
                if let Ok(event) = serde_json::from_str::<LivingProtocolEvent>(&json) {
                    let type_name = get_event_type_name(&event);

                    // Apply filter if specified
                    if let Some(ref filter) = event_type {
                        if !type_name.eq_ignore_ascii_case(filter) {
                            continue;
                        }
                    }

                    yield ProtocolEvent {
                        event_type: type_name,
                        payload: json.clone(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };
                }
            }
        })
    }
}

/// Get the type name of an event for filtering.
fn get_event_type_name(event: &LivingProtocolEvent) -> String {
    match event {
        LivingProtocolEvent::CompostingStarted(_) => "CompostingStarted".to_string(),
        LivingProtocolEvent::NutrientExtracted(_) => "NutrientExtracted".to_string(),
        LivingProtocolEvent::CompostingCompleted(_) => "CompostingCompleted".to_string(),
        LivingProtocolEvent::WoundCreated(_) => "WoundCreated".to_string(),
        LivingProtocolEvent::WoundPhaseAdvanced(_) => "WoundPhaseAdvanced".to_string(),
        LivingProtocolEvent::RestitutionFulfilled(_) => "RestitutionFulfilled".to_string(),
        LivingProtocolEvent::ScarTissueFormed(_) => "ScarTissueFormed".to_string(),
        LivingProtocolEvent::MetabolicTrustUpdated(_) => "MetabolicTrustUpdated".to_string(),
        LivingProtocolEvent::KenosisCommitted(_) => "KenosisCommitted".to_string(),
        LivingProtocolEvent::KenosisExecuted(_) => "KenosisExecuted".to_string(),
        LivingProtocolEvent::TemporalKVectorUpdated(_) => "TemporalKVectorUpdated".to_string(),
        LivingProtocolEvent::FieldInterferenceDetected(_) => {
            "FieldInterferenceDetected".to_string()
        }
        LivingProtocolEvent::DreamStateChanged(_) => "DreamStateChanged".to_string(),
        LivingProtocolEvent::DreamProposalGenerated(_) => "DreamProposalGenerated".to_string(),
        LivingProtocolEvent::NetworkPhiComputed(_) => "NetworkPhiComputed".to_string(),
        LivingProtocolEvent::ShadowSurfaced(_) => "ShadowSurfaced".to_string(),
        LivingProtocolEvent::ClaimHeldInUncertainty(_) => "ClaimHeldInUncertainty".to_string(),
        LivingProtocolEvent::ClaimReleasedFromUncertainty(_) => {
            "ClaimReleasedFromUncertainty".to_string()
        }
        LivingProtocolEvent::SilenceDetected(_) => "SilenceDetected".to_string(),
        LivingProtocolEvent::BeautyScored(_) => "BeautyScored".to_string(),
        LivingProtocolEvent::EntanglementFormed(_) => "EntanglementFormed".to_string(),
        LivingProtocolEvent::EntanglementDecayed(_) => "EntanglementDecayed".to_string(),
        LivingProtocolEvent::AttractorFieldComputed(_) => "AttractorFieldComputed".to_string(),
        LivingProtocolEvent::LiminalTransitionStarted(_) => "LiminalTransitionStarted".to_string(),
        LivingProtocolEvent::LiminalTransitionCompleted(_) => {
            "LiminalTransitionCompleted".to_string()
        }
        LivingProtocolEvent::InterSpeciesRegistered(_) => "InterSpeciesRegistered".to_string(),
        LivingProtocolEvent::ResonanceAddressCreated(_) => "ResonanceAddressCreated".to_string(),
        LivingProtocolEvent::FractalPatternReplicated(_) => "FractalPatternReplicated".to_string(),
        LivingProtocolEvent::MorphogeneticFieldUpdated(_) => {
            "MorphogeneticFieldUpdated".to_string()
        }
        LivingProtocolEvent::TimeCrystalPeriodStarted(_) => "TimeCrystalPeriodStarted".to_string(),
        LivingProtocolEvent::MycelialTaskDistributed(_) => "MycelialTaskDistributed".to_string(),
        LivingProtocolEvent::MycelialTaskCompleted(_) => "MycelialTaskCompleted".to_string(),
        LivingProtocolEvent::PhaseTransitioned(_) => "PhaseTransitioned".to_string(),
        LivingProtocolEvent::CycleStarted(_) => "CycleStarted".to_string(),
    }
}

/// GraphQL schema type.
pub type GraphQLSchema = Schema<QueryRoot, EmptyMutation, SubscriptionRoot>;

/// Configuration for the GraphQL server.
#[derive(Debug, Clone)]
pub struct GraphQLConfig {
    /// Port to listen on for GraphQL
    pub port: u16,
    /// Host to bind to
    pub host: String,
    /// Enable GraphQL Playground
    pub playground: bool,
    /// Enable introspection
    pub introspection: bool,
}

impl Default for GraphQLConfig {
    fn default() -> Self {
        Self {
            port: 8891,
            host: "127.0.0.1".to_string(),
            playground: true,
            introspection: true,
        }
    }
}

/// Create the GraphQL schema with engine context.
pub fn create_schema(
    engine: Arc<RwLock<MetabolismCycleEngine>>,
    event_tx: broadcast::Sender<String>,
) -> GraphQLSchema {
    Schema::build(QueryRoot, EmptyMutation, SubscriptionRoot)
        .data(engine)
        .data(event_tx)
        .finish()
}

/// Run the GraphQL server using Axum.
pub async fn run_graphql_server(
    config: GraphQLConfig,
    engine: Arc<RwLock<MetabolismCycleEngine>>,
    event_tx: broadcast::Sender<String>,
) -> anyhow::Result<()> {
    use async_graphql_axum::GraphQL;
    use axum::{
        response::{Html, IntoResponse},
        routing::get,
        Router,
    };

    let schema = create_schema(engine, event_tx);

    // GraphQL Playground HTML
    async fn graphql_playground() -> impl IntoResponse {
        Html(
            async_graphql::http::playground_source(
                async_graphql::http::GraphQLPlaygroundConfig::new("/graphql")
                    .subscription_endpoint("/graphql/ws"),
            ),
        )
    }

    // GraphQL handler using the GraphQL service (handles both query/mutation and subscriptions)
    let graphql_handler = GraphQL::new(schema);

    let app = Router::new()
        .route("/graphql", get(graphql_playground).post_service(graphql_handler.clone()))
        .route_service("/graphql/ws", graphql_handler);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!(address = %addr, "GraphQL server listening");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_conversion() {
        assert_eq!(Phase::from(CyclePhase::Shadow), Phase::Shadow);
        assert_eq!(CyclePhase::from(Phase::Shadow), CyclePhase::Shadow);

        assert_eq!(Phase::from(CyclePhase::Kenosis), Phase::Kenosis);
        assert_eq!(CyclePhase::from(Phase::Kenosis), CyclePhase::Kenosis);
    }

    #[test]
    fn test_get_event_type_name() {
        let event = LivingProtocolEvent::CycleStarted(living_core::CycleStartedEvent {
            cycle_number: 1,
            started_at: chrono::Utc::now(),
        });
        assert_eq!(get_event_type_name(&event), "CycleStarted");
    }

    #[test]
    fn test_graphql_config_default() {
        let config = GraphQLConfig::default();
        assert_eq!(config.port, 8891);
        assert!(config.playground);
        assert!(config.introspection);
    }
}
