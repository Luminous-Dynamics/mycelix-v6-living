//! Scheduler for the Metabolism Cycle Engine.
//!
//! Provides async scheduling of phase transitions and periodic ticks.

use std::sync::Arc;
use tokio::sync::Mutex;

use living_core::{LivingProtocolEvent, LivingResult, LivingProtocolConfig};
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

    /// Build the engine with all default phase handlers.
    pub fn build(self) -> MetabolismCycleEngine {
        use crate::phase_handlers::*;
        use living_core::CyclePhase;

        let mut engine = MetabolismCycleEngine::new(self.config);

        engine.register_handler(
            CyclePhase::Shadow,
            Box::new(ShadowPhaseHandler::default()),
        );
        engine.register_handler(
            CyclePhase::Composting,
            Box::new(CompostingPhaseHandler::default()),
        );
        engine.register_handler(
            CyclePhase::Liminal,
            Box::new(LiminalPhaseHandler::default()),
        );
        engine.register_handler(
            CyclePhase::NegativeCapability,
            Box::new(NegativeCapabilityPhaseHandler::default()),
        );
        engine.register_handler(
            CyclePhase::Eros,
            Box::new(ErosPhaseHandler::default()),
        );
        engine.register_handler(
            CyclePhase::CoCreation,
            Box::new(CoCreationPhaseHandler::default()),
        );
        engine.register_handler(
            CyclePhase::Beauty,
            Box::new(BeautyPhaseHandler::default()),
        );
        engine.register_handler(
            CyclePhase::EmergentPersonhood,
            Box::new(EmergentPersonhoodPhaseHandler::default()),
        );
        engine.register_handler(
            CyclePhase::Kenosis,
            Box::new(KenosisPhaseHandler::default()),
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
}
