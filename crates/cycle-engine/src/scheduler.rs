//! Scheduler for the Metabolism Cycle Engine.
//!
//! Provides async scheduling of phase transitions and periodic ticks.

use std::sync::Arc;
use tokio::sync::Mutex;

use living_core::{LivingProtocolEvent, LivingResult, LivingProtocolConfig, InMemoryEventBus, EventBus};
use crate::engine::MetabolismCycleEngine;

/// Async scheduler that drives the metabolism cycle engine.
pub struct CycleScheduler {
    engine: Arc<Mutex<MetabolismCycleEngine>>,
    tick_interval: std::time::Duration,
    event_callback: Option<Box<dyn Fn(Vec<LivingProtocolEvent>) + Send + Sync>>,
}

impl CycleScheduler {
    /// Create a new scheduler wrapping an engine.
    pub fn new(engine: MetabolismCycleEngine, tick_interval_secs: u64) -> Self {
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
    pub async fn run(&self) -> LivingResult<()> {
        {
            let mut engine = self.engine.lock().await;
            let start_event = engine.start()?;
            if let Some(ref callback) = self.event_callback {
                callback(vec![LivingProtocolEvent::CycleStarted(start_event)]);
            }
        }

        loop {
            tokio::time::sleep(self.tick_interval).await;

            let mut engine = self.engine.lock().await;
            if !engine.is_running() {
                break;
            }

            match engine.tick() {
                Ok(events) => {
                    if !events.is_empty() {
                        if let Some(ref callback) = self.event_callback {
                            callback(events);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Cycle engine tick error");
                }
            }
        }

        Ok(())
    }

    /// Stop the scheduler.
    pub async fn stop(&self) {
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
    pub fn new() -> Self {
        Self {
            config: LivingProtocolConfig::default(),
        }
    }

    pub fn with_config(mut self, config: LivingProtocolConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_simulated_time(mut self, acceleration: f64) -> Self {
        self.config.cycle.simulated_time = true;
        self.config.cycle.time_acceleration = acceleration;
        self
    }

    /// Build the engine with all phase handlers wired to their primitive engines.
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
}
