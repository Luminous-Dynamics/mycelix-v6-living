//! The Metabolism Cycle State Machine.
//!
//! Orchestrates all 21 primitives through the 28-day lunar cycle.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tracing::info;

use living_core::{
    CyclePhase, CycleState, PhaseTransition, PhaseMetrics,
    LivingProtocolEvent, PhaseTransitionedEvent, CycleStartedEvent,
    LivingProtocolConfig,
    LivingProtocolError, LivingResult,
};

use crate::phase_handlers::PhaseHandler;

/// The main Metabolism Cycle Engine.
///
/// Manages phase transitions, dispatches events to primitives,
/// and enforces phase-specific behavior constraints.
pub struct MetabolismCycleEngine {
    /// Current cycle state
    state: CycleState,
    /// Configuration
    config: LivingProtocolConfig,
    /// Phase handlers (one per phase)
    phase_handlers: HashMap<CyclePhase, Box<dyn PhaseHandler>>,
    /// Event history for current cycle
    cycle_events: Vec<LivingProtocolEvent>,
    /// Phase transition history
    transition_history: Vec<PhaseTransition>,
    /// Whether the engine is running
    running: bool,
}

impl MetabolismCycleEngine {
    /// Create a new engine with default configuration.
    pub fn new(config: LivingProtocolConfig) -> Self {
        let now = Utc::now();
        Self {
            state: CycleState {
                cycle_number: 0,
                current_phase: CyclePhase::Shadow,
                phase_started: now,
                cycle_started: now,
                phase_day: 0,
            },
            config,
            phase_handlers: HashMap::new(),
            cycle_events: Vec::new(),
            transition_history: Vec::new(),
            running: false,
        }
    }

    /// Register a phase handler.
    pub fn register_handler(&mut self, phase: CyclePhase, handler: Box<dyn PhaseHandler>) {
        self.phase_handlers.insert(phase, handler);
    }

    /// Start the engine.
    pub fn start(&mut self) -> LivingResult<CycleStartedEvent> {
        if self.running {
            return Err(LivingProtocolError::PhaseRestriction {
                phase: self.state.current_phase,
                reason: "Engine already running".into(),
            });
        }

        self.running = true;
        let now = self.current_time();
        self.state = CycleState {
            cycle_number: 1,
            current_phase: CyclePhase::Shadow,
            phase_started: now,
            cycle_started: now,
            phase_day: 0,
        };

        let event = CycleStartedEvent {
            cycle_number: 1,
            started_at: now,
        };

        info!(
            cycle = 1,
            phase = ?CyclePhase::Shadow,
            "Metabolism cycle engine started"
        );

        // Notify the Shadow phase handler
        if let Some(handler) = self.phase_handlers.get_mut(&CyclePhase::Shadow) {
            let phase_events = handler.on_enter(&self.state)?;
            for event in phase_events {
                self.cycle_events.push(event);
            }
        }

        Ok(event)
    }

    /// Tick the engine. Should be called periodically.
    /// Checks if the current phase has expired and transitions if needed.
    pub fn tick(&mut self) -> LivingResult<Vec<LivingProtocolEvent>> {
        if !self.running {
            return Err(LivingProtocolError::CycleNotInitialized);
        }

        let now = self.current_time();
        let mut events = Vec::new();

        // Check if current phase has expired
        if self.state.phase_expired(now) {
            let transition_events = self.transition_to_next_phase()?;
            events.extend(transition_events);
        } else {
            // Run the current phase handler's tick
            if let Some(handler) = self.phase_handlers.get_mut(&self.state.current_phase) {
                let tick_events = handler.on_tick(&self.state)?;
                events.extend(tick_events.clone());
                self.cycle_events.extend(tick_events);
            }
        }

        // Update phase day
        let elapsed = now - self.state.phase_started;
        self.state.phase_day = elapsed.num_days() as u32;

        Ok(events)
    }

    /// Force transition to the next phase (for testing or emergency override).
    pub fn force_transition(&mut self) -> LivingResult<Vec<LivingProtocolEvent>> {
        if !self.running {
            return Err(LivingProtocolError::CycleNotInitialized);
        }
        self.transition_to_next_phase()
    }

    /// Get current cycle state.
    pub fn current_state(&self) -> &CycleState {
        &self.state
    }

    /// Get current phase.
    pub fn current_phase(&self) -> CyclePhase {
        self.state.current_phase
    }

    /// Get current cycle number.
    pub fn cycle_number(&self) -> u64 {
        self.state.cycle_number
    }

    /// Check if an operation is permitted in the current phase.
    pub fn is_operation_permitted(&self, operation: &str) -> bool {
        match self.state.current_phase {
            CyclePhase::NegativeCapability => {
                // Voting blocked during Negative Capability phase
                operation != "vote"
            }
            CyclePhase::Shadow => {
                // Gate 2 warnings suspended during Shadow phase
                operation != "gate2_warning"
            }
            CyclePhase::Kenosis => {
                // Only kenosis operations during Kenosis phase
                operation == "kenosis" || operation == "read"
            }
            _ => true,
        }
    }

    /// Check if financial operations are blocked (during Dreaming).
    pub fn is_financial_blocked(&self) -> bool {
        // Financial operations blocked during certain phases
        matches!(
            self.state.current_phase,
            CyclePhase::Kenosis | CyclePhase::EmergentPersonhood
        )
    }

    /// Get time remaining in current phase.
    pub fn time_remaining(&self) -> Duration {
        let now = self.current_time();
        self.state.time_remaining(now)
    }

    /// Get transition history.
    pub fn transition_history(&self) -> &[PhaseTransition] {
        &self.transition_history
    }

    /// Get all events from the current cycle.
    pub fn cycle_events(&self) -> &[LivingProtocolEvent] {
        &self.cycle_events
    }

    /// Whether the engine is running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Stop the engine.
    pub fn stop(&mut self) {
        self.running = false;
        info!("Metabolism cycle engine stopped");
    }

    // =========================================================================
    // Internal Methods
    // =========================================================================

    /// Transition to the next phase.
    fn transition_to_next_phase(&mut self) -> LivingResult<Vec<LivingProtocolEvent>> {
        let now = self.current_time();
        let from = self.state.current_phase;
        let to = from.next();

        info!(from = ?from, to = ?to, cycle = self.state.cycle_number, "Phase transition");

        let mut events = Vec::new();

        // 1. Run exit handler for current phase
        if let Some(handler) = self.phase_handlers.get_mut(&from) {
            let exit_events = handler.on_exit(&self.state)?;
            events.extend(exit_events);
        }

        // 2. Collect metrics at transition
        let metrics = self.collect_phase_metrics();

        // 3. Record the transition
        let transition = PhaseTransition {
            from,
            to,
            cycle_number: self.state.cycle_number,
            transitioned_at: now,
            metrics,
        };
        self.transition_history.push(transition.clone());

        // 4. Check if this completes a full cycle
        if to == CyclePhase::Shadow && from == CyclePhase::Kenosis {
            self.state.cycle_number += 1;
            self.state.cycle_started = now;
            self.cycle_events.clear();

            info!(cycle = self.state.cycle_number, "New metabolism cycle started");

            events.push(LivingProtocolEvent::CycleStarted(CycleStartedEvent {
                cycle_number: self.state.cycle_number,
                started_at: now,
            }));
        }

        // 5. Update state
        self.state.current_phase = to;
        self.state.phase_started = now;
        self.state.phase_day = 0;

        // 6. Run enter handler for new phase
        if let Some(handler) = self.phase_handlers.get_mut(&to) {
            let enter_events = handler.on_enter(&self.state)?;
            events.extend(enter_events);
        }

        // 7. Emit transition event
        events.push(LivingProtocolEvent::PhaseTransitioned(
            PhaseTransitionedEvent {
                transition,
                timestamp: now,
            },
        ));

        self.cycle_events.extend(events.clone());

        Ok(events)
    }

    /// Collect metrics for the phase transition.
    fn collect_phase_metrics(&self) -> PhaseMetrics {
        // Default metrics - each phase handler can provide its own
        let mut metrics = PhaseMetrics {
            active_agents: 0,
            spectral_k: 0.0,
            mean_metabolic_trust: 0.0,
            active_wounds: 0,
            composting_entities: 0,
            liminal_entities: 0,
            entangled_pairs: 0,
            held_uncertainties: 0,
        };

        // Let the current phase handler contribute metrics
        if let Some(handler) = self.phase_handlers.get(&self.state.current_phase) {
            if let Ok(handler_metrics) = serde_json::from_value::<PhaseMetrics>(
                handler.collect_metrics(),
            ) {
                metrics = handler_metrics;
            }
        }

        metrics
    }

    /// Get current time (respects simulated time configuration).
    fn current_time(&self) -> DateTime<Utc> {
        if self.config.cycle.simulated_time {
            // In simulated mode, time is accelerated
            let real_elapsed = Utc::now() - self.state.cycle_started;
            let accelerated = Duration::milliseconds(
                (real_elapsed.num_milliseconds() as f64 * self.config.cycle.time_acceleration)
                    as i64,
            );
            self.state.cycle_started + accelerated
        } else {
            Utc::now()
        }
    }
}

// =============================================================================
// Phase-Specific Behavior Validators
// =============================================================================

/// Validates that an operation is permitted given the current cycle phase.
pub struct PhaseValidator;

impl PhaseValidator {
    /// Check if voting is permitted.
    pub fn can_vote(state: &CycleState) -> bool {
        !matches!(state.current_phase, CyclePhase::NegativeCapability)
    }

    /// Check if Gate 2 warnings are active.
    pub fn gate2_active(state: &CycleState) -> bool {
        // Gate 2 warnings suspended during Shadow phase
        !matches!(state.current_phase, CyclePhase::Shadow)
    }

    /// Check if composting is active.
    pub fn can_compost(state: &CycleState) -> bool {
        matches!(state.current_phase, CyclePhase::Composting)
    }

    /// Check if kenosis is permitted.
    pub fn can_kenosis(state: &CycleState) -> bool {
        matches!(state.current_phase, CyclePhase::Kenosis)
    }

    /// Check if beauty scoring is active.
    pub fn can_beauty_score(state: &CycleState) -> bool {
        matches!(state.current_phase, CyclePhase::Beauty)
    }

    /// Check if liminal transitions are active.
    pub fn can_enter_liminal(state: &CycleState) -> bool {
        matches!(state.current_phase, CyclePhase::Liminal)
    }

    /// Check if eros/attractor computation is active.
    pub fn can_compute_attractors(state: &CycleState) -> bool {
        matches!(state.current_phase, CyclePhase::Eros)
    }

    /// Check if co-creation (standard consensus) is active.
    pub fn can_co_create(state: &CycleState) -> bool {
        matches!(state.current_phase, CyclePhase::CoCreation)
    }

    /// Check if emergent personhood measurement is active.
    pub fn can_measure_personhood(state: &CycleState) -> bool {
        matches!(state.current_phase, CyclePhase::EmergentPersonhood)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> LivingProtocolConfig {
        let mut config = LivingProtocolConfig::default();
        config.cycle.simulated_time = true;
        config.cycle.time_acceleration = 86400.0; // 1 second = 1 day
        config
    }

    #[test]
    fn test_engine_start() {
        let mut engine = MetabolismCycleEngine::new(test_config());
        let event = engine.start().unwrap();

        assert_eq!(event.cycle_number, 1);
        assert_eq!(engine.current_phase(), CyclePhase::Shadow);
        assert!(engine.is_running());
    }

    #[test]
    fn test_engine_double_start_fails() {
        let mut engine = MetabolismCycleEngine::new(test_config());
        engine.start().unwrap();
        assert!(engine.start().is_err());
    }

    #[test]
    fn test_force_transition_sequence() {
        let mut engine = MetabolismCycleEngine::new(test_config());
        engine.start().unwrap();

        // Walk through all 9 phases
        let expected = [
            CyclePhase::Composting,
            CyclePhase::Liminal,
            CyclePhase::NegativeCapability,
            CyclePhase::Eros,
            CyclePhase::CoCreation,
            CyclePhase::Beauty,
            CyclePhase::EmergentPersonhood,
            CyclePhase::Kenosis,
            CyclePhase::Shadow, // Back to start
        ];

        for expected_phase in &expected {
            engine.force_transition().unwrap();
            assert_eq!(engine.current_phase(), *expected_phase);
        }

        // After full cycle, cycle number should be 2
        assert_eq!(engine.cycle_number(), 2);
    }

    #[test]
    fn test_phase_validator_voting_blocked() {
        let state = CycleState {
            cycle_number: 1,
            current_phase: CyclePhase::NegativeCapability,
            phase_started: Utc::now(),
            cycle_started: Utc::now(),
            phase_day: 0,
        };
        assert!(!PhaseValidator::can_vote(&state));
    }

    #[test]
    fn test_phase_validator_voting_allowed() {
        let state = CycleState {
            cycle_number: 1,
            current_phase: CyclePhase::CoCreation,
            phase_started: Utc::now(),
            cycle_started: Utc::now(),
            phase_day: 0,
        };
        assert!(PhaseValidator::can_vote(&state));
    }

    #[test]
    fn test_phase_validator_gate2_suspended_in_shadow() {
        let state = CycleState {
            cycle_number: 1,
            current_phase: CyclePhase::Shadow,
            phase_started: Utc::now(),
            cycle_started: Utc::now(),
            phase_day: 0,
        };
        assert!(!PhaseValidator::gate2_active(&state));
    }

    #[test]
    fn test_phase_validator_kenosis_only_in_kenosis_phase() {
        for phase in CyclePhase::all_phases() {
            let state = CycleState {
                cycle_number: 1,
                current_phase: *phase,
                phase_started: Utc::now(),
                cycle_started: Utc::now(),
                phase_day: 0,
            };
            if *phase == CyclePhase::Kenosis {
                assert!(PhaseValidator::can_kenosis(&state));
            } else {
                assert!(!PhaseValidator::can_kenosis(&state));
            }
        }
    }

    #[test]
    fn test_operation_permitted() {
        let engine = MetabolismCycleEngine::new(test_config());
        // Default state is Shadow
        assert!(engine.is_operation_permitted("vote"));
        assert!(!engine.is_operation_permitted("gate2_warning"));
    }

    #[test]
    fn test_transition_history_recorded() {
        let mut engine = MetabolismCycleEngine::new(test_config());
        engine.start().unwrap();

        engine.force_transition().unwrap();
        engine.force_transition().unwrap();

        assert_eq!(engine.transition_history().len(), 2);
        assert_eq!(engine.transition_history()[0].from, CyclePhase::Shadow);
        assert_eq!(engine.transition_history()[0].to, CyclePhase::Composting);
        assert_eq!(engine.transition_history()[1].from, CyclePhase::Composting);
        assert_eq!(engine.transition_history()[1].to, CyclePhase::Liminal);
    }

    #[test]
    fn test_full_cycle_transitions_correct() {
        let mut engine = MetabolismCycleEngine::new(test_config());
        engine.start().unwrap();

        // Complete one full cycle (9 transitions)
        for _ in 0..9 {
            engine.force_transition().unwrap();
        }

        assert_eq!(engine.transition_history().len(), 9);
        assert_eq!(engine.cycle_number(), 2);
        assert_eq!(engine.current_phase(), CyclePhase::Shadow);
    }
}
