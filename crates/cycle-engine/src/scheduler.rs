//! Scheduler for the Metabolism Cycle Engine.
//!
//! Provides async scheduling of phase transitions and periodic ticks.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, instrument};

use living_core::{LivingProtocolEvent, LivingResult, LivingProtocolConfig, InMemoryEventBus, EventBus};
use crate::engine::MetabolismCycleEngine;

#[cfg(feature = "telemetry")]
use crate::telemetry::metrics;

/// Async scheduler that drives the metabolism cycle engine.
pub struct CycleScheduler {
    engine: Arc<Mutex<MetabolismCycleEngine>>,
    tick_interval: std::time::Duration,
    event_callback: Option<Box<dyn Fn(Vec<LivingProtocolEvent>) + Send + Sync>>,
}

impl CycleScheduler {
    /// Create a new scheduler wrapping an engine.
    #[instrument(skip(engine), fields(tick_interval_secs = tick_interval_secs))]
    pub fn new(engine: MetabolismCycleEngine, tick_interval_secs: u64) -> Self {
        info!("Creating new CycleScheduler");
        Self {
            engine: Arc::new(Mutex::new(engine)),
            tick_interval: std::time::Duration::from_secs(tick_interval_secs),
            event_callback: None,
        }
    }

    /// Set a callback for events.
    pub fn on_events(mut self, callback: impl Fn(Vec<LivingProtocolEvent>) + Send + Sync + 'static) -> Self {
        self.event_callback = Some(Box::new(callback));
        self
    }

    /// Get a clone of the engine reference (for external inspection).
    pub fn engine(&self) -> Arc<Mutex<MetabolismCycleEngine>> {
        self.engine.clone()
    }

    /// Start the engine and run the tick loop.
    #[instrument(skip(self), name = "cycle_scheduler_run")]
    pub async fn run(&self) -> LivingResult<()> {
        #[cfg(feature = "telemetry")]
        let cycle_start = std::time::Instant::now();

        {
            let mut engine = self.engine.lock().await;
            let start_event = engine.start()?;
            info!(
                cycle = start_event.cycle_number,
                "Cycle engine started"
            );
            if let Some(ref callback) = self.event_callback {
                callback(vec![LivingProtocolEvent::CycleStarted(start_event)]);
            }
        }

        loop {
            tokio::time::sleep(self.tick_interval).await;

            let mut engine = self.engine.lock().await;
            if !engine.is_running() {
                info!("Cycle engine stopped, exiting scheduler loop");
                break;
            }

            // Capture state before tick for telemetry
            let phase = engine.current_phase();
            #[cfg(feature = "telemetry")]
            let phase_day = engine.current_state().phase_day;
            let cycle_number = engine.cycle_number();

            #[cfg(feature = "telemetry")]
            let tick_start = std::time::Instant::now();

            match engine.tick() {
                Ok(events) => {
                    #[cfg(feature = "telemetry")]
                    {
                        let tick_duration = tick_start.elapsed();
                        if let Some(m) = metrics() {
                            m.record_tick_duration(
                                tick_duration.as_secs_f64() * 1000.0,
                                phase,
                                phase_day,
                            );
                        }
                    }

                    // Check for phase transitions and record metrics
                    for event in &events {
                        if let LivingProtocolEvent::PhaseTransitioned(transition_event) = event {
                            let span = tracing::info_span!(
                                "phase_transition",
                                from_phase = ?transition_event.transition.from,
                                to_phase = ?transition_event.transition.to,
                                cycle = transition_event.transition.cycle_number,
                            );
                            let _enter = span.enter();
                            info!(
                                from = ?transition_event.transition.from,
                                to = ?transition_event.transition.to,
                                "Phase transition completed"
                            );

                            #[cfg(feature = "telemetry")]
                            if let Some(m) = metrics() {
                                m.record_phase_transition(
                                    transition_event.transition.from,
                                    transition_event.transition.to,
                                    transition_event.transition.cycle_number,
                                );
                            }
                        }

                        // Track cycle completions (when we transition back to Shadow)
                        if let LivingProtocolEvent::CycleStarted(cycle_event) = event {
                            info!(
                                new_cycle = cycle_event.cycle_number,
                                "New cycle started"
                            );

                            #[cfg(feature = "telemetry")]
                            if cycle_event.cycle_number > 1 {
                                if let Some(m) = metrics() {
                                    let cycle_duration = cycle_start.elapsed();
                                    m.record_cycle_completion(
                                        cycle_duration.as_secs_f64(),
                                        cycle_event.cycle_number - 1,
                                    );
                                }
                            }
                        }
                    }

                    if !events.is_empty() {
                        if let Some(ref callback) = self.event_callback {
                            callback(events);
                        }
                    }
                }
                Err(e) => {
                    error!(
                        error = %e,
                        phase = ?phase,
                        cycle = cycle_number,
                        "Cycle engine tick error"
                    );

                    #[cfg(feature = "telemetry")]
                    if let Some(m) = metrics() {
                        m.record_tick_error(phase, &e.to_string());
                    }
                }
            }
        }

        Ok(())
    }

    /// Stop the scheduler.
    #[instrument(skip(self), name = "cycle_scheduler_stop")]
    pub async fn stop(&self) {
        info!("Stopping cycle scheduler");
        let mut engine = self.engine.lock().await;
        engine.stop();
    }
}

/// Builder for creating a fully configured cycle engine with all phase handlers.
///
/// Creates shared `InMemoryEventBus` and `LivingProtocolConfig`, then constructs
/// each phase handler with its corresponding primitive engine wired in.
pub struct CycleEngineBuilder {
    config: LivingProtocolConfig,
}

impl CycleEngineBuilder {
    /// Create a new CycleEngineBuilder with default configuration.
    #[instrument(name = "cycle_engine_builder_new")]
    pub fn new() -> Self {
        info!("Creating new CycleEngineBuilder");
        Self {
            config: LivingProtocolConfig::default(),
        }
    }

    /// Set a custom configuration.
    pub fn with_config(mut self, config: LivingProtocolConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable simulated time with the given acceleration factor.
    #[instrument(skip(self), fields(acceleration = acceleration))]
    pub fn with_simulated_time(mut self, acceleration: f64) -> Self {
        info!(acceleration = acceleration, "Enabling simulated time");
        self.config.cycle.simulated_time = true;
        self.config.cycle.time_acceleration = acceleration;
        self
    }

    /// Build the engine with all phase handlers wired to their primitive engines.
    #[instrument(skip(self), name = "cycle_engine_build")]
    pub fn build(self) -> MetabolismCycleEngine {
        use crate::phase_handlers::*;
        use living_core::CyclePhase;

        // Shared event bus for engines that need one
        let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());

        let mut engine = MetabolismCycleEngine::new(self.config.clone());

        // Shadow: ShadowIntegrationEngine (no deps)
        engine.register_handler(
            CyclePhase::Shadow,
            Box::new(ShadowPhaseHandler::new(
                self.config.shadow.spectral_k_anomaly_threshold,
                self.config.shadow.clone(),
            )),
        );

        // Composting: CompostingEngine (config + event_bus)
        engine.register_handler(
            CyclePhase::Composting,
            Box::new(CompostingPhaseHandler::new(
                self.config.composting.clone(),
                event_bus.clone(),
            )),
        );

        // Liminal: LiminalityEngine (no deps)
        engine.register_handler(
            CyclePhase::Liminal,
            Box::new(LiminalPhaseHandler::new()),
        );

        // Negative Capability: NegativeCapabilityEngine (no deps)
        engine.register_handler(
            CyclePhase::NegativeCapability,
            Box::new(NegativeCapabilityPhaseHandler::new(
                self.config.negative_capability.clone(),
            )),
        );

        // Eros: ErosAttractorEngine (feature flags)
        engine.register_handler(
            CyclePhase::Eros,
            Box::new(ErosPhaseHandler::new(self.config.features.clone())),
        );

        // Co-Creation: EntanglementEngine (config)
        engine.register_handler(
            CyclePhase::CoCreation,
            Box::new(CoCreationPhaseHandler::new(
                self.config.entanglement.clone(),
            )),
        );

        // Beauty: BeautyValidityEngine (no deps)
        engine.register_handler(
            CyclePhase::Beauty,
            Box::new(BeautyPhaseHandler::new()),
        );

        // Emergent Personhood: EmergentPersonhoodService (tier4-aspirational feature)
        engine.register_handler(
            CyclePhase::EmergentPersonhood,
            Box::new(EmergentPersonhoodPhaseHandler::new()),
        );

        // Kenosis: KenosisEngine (config + event_bus)
        engine.register_handler(
            CyclePhase::Kenosis,
            Box::new(KenosisPhaseHandler::new(
                self.config.kenosis.clone(),
                event_bus,
            )),
        );

        info!("Cycle engine built with all phase handlers registered");
        engine
    }
}

impl Default for CycleEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase_handlers::PhaseHandler;

    fn make_state(cycle: u64, phase: living_core::CyclePhase, phase_day: u32) -> living_core::CycleState {
        living_core::CycleState {
            cycle_number: cycle,
            current_phase: phase,
            phase_started: chrono::Utc::now(),
            cycle_started: chrono::Utc::now(),
            phase_day,
        }
    }

    #[test]
    fn test_builder_creates_engine_with_all_handlers() {
        let engine = CycleEngineBuilder::new()
            .with_simulated_time(86400.0)
            .build();

        assert!(!engine.is_running());
    }

    #[test]
    fn test_builder_full_cycle() {
        let mut engine = CycleEngineBuilder::new()
            .with_simulated_time(86400.0)
            .build();

        engine.start().unwrap();

        // Walk through all 9 transitions
        for _ in 0..9 {
            let events = engine.force_transition().unwrap();
            assert!(!events.is_empty()); // At least the transition event
        }

        assert_eq!(engine.cycle_number(), 2);
    }

    #[test]
    fn test_builder_with_custom_config() {
        let mut config = LivingProtocolConfig::default();
        config.shadow.spectral_k_anomaly_threshold = 0.5;
        config.kenosis.max_release_per_cycle = 0.10;

        let engine = CycleEngineBuilder::new()
            .with_config(config)
            .with_simulated_time(86400.0)
            .build();

        assert!(!engine.is_running());
    }

    // =========================================================================
    // Integration tests for phase handler <-> engine wiring
    // =========================================================================

    #[test]
    fn test_shadow_handler_surfaces_suppressed_content() {
        use crate::phase_handlers::ShadowPhaseHandler;
        use living_core::ShadowConfig;

        let mut handler = ShadowPhaseHandler::new(0.3, ShadowConfig::default());

        // Record some suppressed content via the engine
        handler.engine_mut().record_suppression(
            "content-1",
            "low quality",
            0.8, // suppressor rep
            0.2, // author rep (low rep dissent)
            false, // not gate1 protected
        );

        let state = make_state(1, living_core::CyclePhase::Shadow, 0);

        // Tick should surface the suppressed content
        let _events = handler.on_tick(&state).unwrap();

        // Verify metrics reflect the engine state
        let metrics = handler.collect_metrics();
        assert!(metrics.get("suppressed_content_count").is_some());
    }

    #[test]
    fn test_negative_capability_auto_releases_on_tick() {
        use crate::phase_handlers::NegativeCapabilityPhaseHandler;
        use living_core::NegativeCapabilityConfig;

        let mut config = NegativeCapabilityConfig::default();
        config.max_hold_days = 0; // Immediate expiry for testing

        let mut handler = NegativeCapabilityPhaseHandler::new(config);

        // Hold a claim via the engine
        handler.engine_mut().hold_in_uncertainty(
            "claim-1",
            "needs more research",
            0, // min hold days
            "did:agent:holder",
        );

        assert!(handler.engine().is_held("claim-1"));
        assert!(!handler.engine().can_vote_on("claim-1"));

        let state = make_state(1, living_core::CyclePhase::NegativeCapability, 0);

        // Tick should auto-release expired claims
        let events = handler.on_tick(&state).unwrap();

        // All claims with max_hold_days=0 should be released
        assert!(!events.is_empty() || handler.engine().held_count() == 0);
    }

    #[test]
    fn test_cocreation_handler_decays_entanglements() {
        use crate::phase_handlers::CoCreationPhaseHandler;
        use living_core::EntanglementConfig;

        let mut config = EntanglementConfig::default();
        config.min_co_creation_events = 1;
        config.decay_rate_per_day = 0.5; // Fast decay for testing

        let mut handler = CoCreationPhaseHandler::new(config);

        // Record co-creation to form entanglement
        handler.engine_mut().record_co_creation(
            &"did:agent:alice".to_string(),
            &"did:agent:bob".to_string(),
            "collaborated on proposal",
            0.9,
        );

        // Form entanglement (requires min_co_creation_events)
        let _ = handler.engine_mut().form_entanglement(
            &"did:agent:alice".to_string(),
            &"did:agent:bob".to_string(),
        );

        let state = make_state(1, living_core::CyclePhase::CoCreation, 3);

        // Tick triggers decay_all
        let _events = handler.on_tick(&state).unwrap();

        // Metrics should reflect the engine state
        let metrics = handler.collect_metrics();
        assert!(metrics.get("phase").is_some());
    }

    #[test]
    fn test_kenosis_handler_sets_cycle_on_enter() {
        use crate::phase_handlers::KenosisPhaseHandler;
        use living_core::{KenosisConfig, InMemoryEventBus};
        use std::sync::Arc;

        let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
        let mut handler = KenosisPhaseHandler::new(KenosisConfig::default(), event_bus);

        let state = make_state(42, living_core::CyclePhase::Kenosis, 0);

        // Enter should set cycle number on engine
        handler.on_enter(&state).unwrap();

        // Register an agent and try to commit kenosis
        handler.engine_mut().register_agent("did:agent:test", 100.0);
        let result = handler.engine_mut().commit_kenosis("did:agent:test", 0.10);
        assert!(result.is_ok());

        // The commitment should be for cycle 42
        let commitment = result.unwrap();
        assert_eq!(commitment.cycle_number, 42);
    }

    #[test]
    fn test_composting_handler_reports_active_composting() {
        use crate::phase_handlers::CompostingPhaseHandler;
        use living_core::{CompostingConfig, InMemoryEventBus, CompostableEntity};
        use std::sync::Arc;

        let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
        let mut handler = CompostingPhaseHandler::new(CompostingConfig::default(), event_bus);

        // Start composting via the engine
        handler.engine_mut().start_composting(
            CompostableEntity::FailedProposal,
            "prop-123".to_string(),
            metabolism::composting::CompostingReason::ProposalFailed {
                vote_count: 5,
                required: 10,
            },
        ).unwrap();

        let state = make_state(1, living_core::CyclePhase::Composting, 2);

        // Tick updates metrics
        handler.on_tick(&state).unwrap();

        let metrics = handler.collect_metrics();
        assert_eq!(metrics["active_composting"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_beauty_handler_tracks_scored_proposals() {
        use crate::phase_handlers::BeautyPhaseHandler;

        let mut handler = BeautyPhaseHandler::new();

        // Score a proposal via the engine
        handler.engine_mut().score_proposal(
            "proposal-1",
            "A well-structured proposal with clear benefits and elegant design.",
            "did:scorer:1",
            &["existing pattern 1".to_string()],
            &["requirement 1".to_string()],
        );

        let state = make_state(1, living_core::CyclePhase::Beauty, 1);

        // Tick updates the proposals_scored count
        handler.on_tick(&state).unwrap();

        let metrics = handler.collect_metrics();
        assert_eq!(metrics["proposals_scored"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_liminal_handler_tracks_entities() {
        use crate::phase_handlers::LiminalPhaseHandler;
        use living_core::LiminalEntityType;

        let mut handler = LiminalPhaseHandler::new();

        // Enter an entity into liminal state via the engine (returns event, not Result)
        let _event = handler.engine_mut().enter_liminal_state(
            &"did:entity:transitioning".to_string(),
            LiminalEntityType::Agent,
            Some("Identity transformation".to_string()),
        );

        let state = make_state(1, living_core::CyclePhase::Liminal, 1);

        // Tick updates count
        handler.on_tick(&state).unwrap();

        let metrics = handler.collect_metrics();
        assert_eq!(metrics["entities_in_transition"].as_u64().unwrap(), 1);
        assert!(handler.engine().is_recategorization_blocked(&"did:entity:transitioning".to_string()));
    }

    #[test]
    fn test_full_cycle_with_wired_handlers() {
        // Build engine with all handlers wired
        let mut engine = CycleEngineBuilder::new()
            .with_simulated_time(86400.0)
            .build();

        engine.start().unwrap();

        // Walk through all phases and verify ticks produce valid events
        let phases = [
            "Shadow", "Composting", "Liminal", "NegativeCapability",
            "Eros", "CoCreation", "Beauty", "EmergentPersonhood", "Kenosis",
        ];

        for (_i, phase_name) in phases.iter().enumerate() {
            // Tick in each phase
            let _tick_events = engine.tick().unwrap();
            // Ticks should succeed (may or may not produce events)

            // Transition to next phase
            let transition_events = engine.force_transition().unwrap();
            assert!(!transition_events.is_empty(), "Phase {} transition should emit events", phase_name);
        }

        // Should be in cycle 2 now
        assert_eq!(engine.cycle_number(), 2);
    }
}
