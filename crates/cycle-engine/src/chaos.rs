//! Chaos testing infrastructure for the Metabolism Cycle Engine.
//!
//! Provides fault injection capabilities to test system resilience:
//! - Handler panic simulation
//! - Clock skew injection
//! - Artificial delays
//! - State corruption detection
//!
//! # Usage
//!
//! ```rust,ignore
//! use cycle_engine::chaos::{ChaosInjector, ChaosConfig};
//!
//! let config = ChaosConfig::default()
//!     .with_panic_on_enter(CyclePhase::Composting)
//!     .with_clock_skew(Duration::hours(1));
//!
//! let mut injector = ChaosInjector::new(config);
//! ```

use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use chrono::{DateTime, Duration, Utc};

use living_core::{CyclePhase, CycleState, LivingProtocolEvent};

/// Configuration for chaos injection scenarios.
#[derive(Debug, Clone, Default)]
pub struct ChaosConfig {
    /// If set, handler will panic when entering this phase.
    pub panic_on_enter: Option<CyclePhase>,

    /// If set, handler will panic when exiting this phase.
    pub panic_on_exit: Option<CyclePhase>,

    /// If set, handler will panic during tick for this phase.
    pub panic_on_tick: Option<CyclePhase>,

    /// If set, introduces random delays in the given range during operations.
    pub delay_range: Option<Range<Duration>>,

    /// If set, skews the reported time by this amount.
    pub clock_skew: Option<Duration>,

    /// If set, causes nth operation to fail.
    pub fail_after_n_operations: Option<u64>,

    /// If true, enables callback reentrancy testing.
    pub enable_reentrancy_test: bool,

    /// If true, simulates time overflow conditions.
    pub simulate_time_overflow: bool,
}

impl ChaosConfig {
    /// Create a new ChaosConfig with default (no chaos) settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure panic on entering a specific phase.
    pub fn with_panic_on_enter(mut self, phase: CyclePhase) -> Self {
        self.panic_on_enter = Some(phase);
        self
    }

    /// Configure panic on exiting a specific phase.
    pub fn with_panic_on_exit(mut self, phase: CyclePhase) -> Self {
        self.panic_on_exit = Some(phase);
        self
    }

    /// Configure panic during tick for a specific phase.
    pub fn with_panic_on_tick(mut self, phase: CyclePhase) -> Self {
        self.panic_on_tick = Some(phase);
        self
    }

    /// Configure random delay injection.
    pub fn with_delay_range(mut self, range: Range<Duration>) -> Self {
        self.delay_range = Some(range);
        self
    }

    /// Configure clock skew.
    pub fn with_clock_skew(mut self, skew: Duration) -> Self {
        self.clock_skew = Some(skew);
        self
    }

    /// Configure failure after N operations.
    pub fn with_fail_after_n(mut self, n: u64) -> Self {
        self.fail_after_n_operations = Some(n);
        self
    }

    /// Enable reentrancy testing.
    pub fn with_reentrancy_test(mut self) -> Self {
        self.enable_reentrancy_test = true;
        self
    }

    /// Enable time overflow simulation.
    pub fn with_time_overflow(mut self) -> Self {
        self.simulate_time_overflow = true;
        self
    }
}

/// Chaos injector for testing fault tolerance.
///
/// Tracks operation counts and injects faults according to configuration.
pub struct ChaosInjector {
    config: ChaosConfig,
    operation_count: AtomicU64,
    is_in_callback: AtomicBool,
    panicked_phases: Vec<CyclePhase>,
}

impl ChaosInjector {
    /// Create a new ChaosInjector with the given configuration.
    pub fn new(config: ChaosConfig) -> Self {
        Self {
            config,
            operation_count: AtomicU64::new(0),
            is_in_callback: AtomicBool::new(false),
            panicked_phases: Vec::new(),
        }
    }

    /// Check if we should inject a panic on phase enter.
    pub fn should_panic_on_enter(&self, phase: CyclePhase) -> bool {
        self.config.panic_on_enter == Some(phase)
    }

    /// Check if we should inject a panic on phase exit.
    pub fn should_panic_on_exit(&self, phase: CyclePhase) -> bool {
        self.config.panic_on_exit == Some(phase)
    }

    /// Check if we should inject a panic on tick.
    pub fn should_panic_on_tick(&self, phase: CyclePhase) -> bool {
        self.config.panic_on_tick == Some(phase)
    }

    /// Increment operation count and check if we should fail.
    pub fn should_fail(&self) -> bool {
        let count = self.operation_count.fetch_add(1, Ordering::SeqCst);
        if let Some(n) = self.config.fail_after_n_operations {
            count >= n
        } else {
            false
        }
    }

    /// Get the current operation count.
    pub fn operation_count(&self) -> u64 {
        self.operation_count.load(Ordering::SeqCst)
    }

    /// Get clock skew if configured.
    pub fn clock_skew(&self) -> Option<Duration> {
        self.config.clock_skew
    }

    /// Apply clock skew to a timestamp.
    pub fn apply_clock_skew(&self, time: DateTime<Utc>) -> DateTime<Utc> {
        if let Some(skew) = self.config.clock_skew {
            time + skew
        } else {
            time
        }
    }

    /// Check if time overflow should be simulated.
    pub fn should_simulate_overflow(&self) -> bool {
        self.config.simulate_time_overflow
    }

    /// Enter a callback context (for reentrancy detection).
    pub fn enter_callback(&self) -> bool {
        if self.config.enable_reentrancy_test {
            // Returns true if we were NOT already in a callback
            !self.is_in_callback.swap(true, Ordering::SeqCst)
        } else {
            true
        }
    }

    /// Exit a callback context.
    pub fn exit_callback(&self) {
        if self.config.enable_reentrancy_test {
            self.is_in_callback.store(false, Ordering::SeqCst);
        }
    }

    /// Check if we're currently in a callback (reentrancy).
    pub fn is_in_callback(&self) -> bool {
        self.is_in_callback.load(Ordering::SeqCst)
    }

    /// Record that a phase panicked (for recovery testing).
    pub fn record_panic(&mut self, phase: CyclePhase) {
        self.panicked_phases.push(phase);
    }

    /// Get phases that have panicked.
    pub fn panicked_phases(&self) -> &[CyclePhase] {
        &self.panicked_phases
    }

    /// Reset the injector state.
    pub fn reset(&mut self) {
        self.operation_count.store(0, Ordering::SeqCst);
        self.is_in_callback.store(false, Ordering::SeqCst);
        self.panicked_phases.clear();
    }
}

/// State checkpoint for transactional operations.
///
/// Used to enable rollback on failure.
#[derive(Clone, Debug)]
pub struct StateCheckpoint {
    /// Cycle state at checkpoint time.
    pub cycle_state: CycleState,

    /// Events accumulated before checkpoint.
    pub events: Vec<LivingProtocolEvent>,

    /// Timestamp of checkpoint.
    pub created_at: DateTime<Utc>,
}

impl StateCheckpoint {
    /// Create a new checkpoint from the current state.
    pub fn new(state: &CycleState, events: Vec<LivingProtocolEvent>) -> Self {
        Self {
            cycle_state: state.clone(),
            events,
            created_at: Utc::now(),
        }
    }
}

/// Result type for chaos-tested operations.
pub type ChaosResult<T> = Result<T, ChaosError>;

/// Errors specific to chaos testing scenarios.
#[derive(Debug, Clone)]
pub enum ChaosError {
    /// Injected panic during phase enter.
    PanicOnEnter(CyclePhase),

    /// Injected panic during phase exit.
    PanicOnExit(CyclePhase),

    /// Injected panic during tick.
    PanicOnTick(CyclePhase),

    /// Failure triggered after N operations.
    FailedAfterN(u64),

    /// Reentrancy detected.
    ReentrancyDetected,

    /// Time overflow detected.
    TimeOverflow,

    /// Rollback required.
    RollbackRequired(String),
}

impl std::fmt::Display for ChaosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChaosError::PanicOnEnter(phase) => write!(f, "Injected panic on entering {:?}", phase),
            ChaosError::PanicOnExit(phase) => write!(f, "Injected panic on exiting {:?}", phase),
            ChaosError::PanicOnTick(phase) => write!(f, "Injected panic during tick in {:?}", phase),
            ChaosError::FailedAfterN(n) => write!(f, "Failed after {} operations", n),
            ChaosError::ReentrancyDetected => write!(f, "Callback reentrancy detected"),
            ChaosError::TimeOverflow => write!(f, "Time calculation overflow"),
            ChaosError::RollbackRequired(reason) => write!(f, "Rollback required: {}", reason),
        }
    }
}

impl std::error::Error for ChaosError {}

/// Wrapper for chaos-enabled phase transitions with rollback support.
pub struct TransactionalTransition {
    checkpoint: Option<StateCheckpoint>,
    committed: bool,
}

impl TransactionalTransition {
    /// Begin a new transactional transition.
    pub fn begin(state: &CycleState, events: Vec<LivingProtocolEvent>) -> Self {
        Self {
            checkpoint: Some(StateCheckpoint::new(state, events)),
            committed: false,
        }
    }

    /// Commit the transition (discard checkpoint).
    pub fn commit(&mut self) {
        self.committed = true;
        self.checkpoint = None;
    }

    /// Rollback to checkpoint.
    pub fn rollback(&self) -> Option<&StateCheckpoint> {
        if !self.committed {
            self.checkpoint.as_ref()
        } else {
            None
        }
    }

    /// Check if the transition was committed.
    pub fn is_committed(&self) -> bool {
        self.committed
    }
}

impl Drop for TransactionalTransition {
    fn drop(&mut self) {
        if !self.committed && self.checkpoint.is_some() {
            tracing::warn!("TransactionalTransition dropped without commit - rollback should be performed");
        }
    }
}

/// Saturating time arithmetic to prevent overflow.
pub fn saturating_add_duration(base: DateTime<Utc>, duration: Duration) -> DateTime<Utc> {
    // Check for potential overflow
    let ms = duration.num_milliseconds();
    if ms > i64::MAX / 2 {
        // Return a far-future timestamp instead of overflowing
        DateTime::<Utc>::MAX_UTC
    } else {
        base.checked_add_signed(duration).unwrap_or(DateTime::<Utc>::MAX_UTC)
    }
}

/// Saturating multiplication for time acceleration.
pub fn saturating_time_acceleration(elapsed_ms: i64, acceleration: f64) -> i64 {
    let accelerated = elapsed_ms as f64 * acceleration;
    if accelerated > i64::MAX as f64 / 2.0 {
        i64::MAX / 2
    } else if accelerated < i64::MIN as f64 / 2.0 {
        i64::MIN / 2
    } else {
        accelerated as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_config_builder() {
        let config = ChaosConfig::new()
            .with_panic_on_enter(CyclePhase::Shadow)
            .with_clock_skew(Duration::hours(1))
            .with_fail_after_n(10);

        assert_eq!(config.panic_on_enter, Some(CyclePhase::Shadow));
        assert_eq!(config.clock_skew, Some(Duration::hours(1)));
        assert_eq!(config.fail_after_n_operations, Some(10));
    }

    #[test]
    fn test_chaos_injector_operation_counting() {
        let config = ChaosConfig::new().with_fail_after_n(3);
        let injector = ChaosInjector::new(config);

        assert!(!injector.should_fail()); // 0
        assert!(!injector.should_fail()); // 1
        assert!(!injector.should_fail()); // 2
        assert!(injector.should_fail());  // 3 - should fail
    }

    #[test]
    fn test_chaos_injector_reentrancy() {
        let config = ChaosConfig::new().with_reentrancy_test();
        let injector = ChaosInjector::new(config);

        assert!(!injector.is_in_callback());
        assert!(injector.enter_callback()); // First entry succeeds
        assert!(injector.is_in_callback());
        assert!(!injector.enter_callback()); // Reentrant call detected
        injector.exit_callback();
        assert!(!injector.is_in_callback());
    }

    #[test]
    fn test_clock_skew_application() {
        let config = ChaosConfig::new().with_clock_skew(Duration::hours(2));
        let injector = ChaosInjector::new(config);

        let now = Utc::now();
        let skewed = injector.apply_clock_skew(now);

        assert_eq!(skewed - now, Duration::hours(2));
    }

    #[test]
    fn test_transactional_transition() {
        let state = CycleState {
            cycle_number: 1,
            current_phase: CyclePhase::Shadow,
            phase_started: Utc::now(),
            cycle_started: Utc::now(),
            phase_day: 0,
        };

        let mut txn = TransactionalTransition::begin(&state, vec![]);
        assert!(!txn.is_committed());
        assert!(txn.rollback().is_some());

        txn.commit();
        assert!(txn.is_committed());
        assert!(txn.rollback().is_none());
    }

    #[test]
    fn test_saturating_time_arithmetic() {
        let base = Utc::now();

        // Normal case
        let result = saturating_add_duration(base, Duration::hours(1));
        assert!(result > base);

        // Overflow case - should saturate
        let huge_duration = Duration::milliseconds(i64::MAX);
        let result = saturating_add_duration(base, huge_duration);
        // Should not panic and should return a valid time
        assert!(result >= base);
    }

    #[test]
    fn test_saturating_time_acceleration() {
        // Normal case
        let result = saturating_time_acceleration(1000, 2.0);
        assert_eq!(result, 2000);

        // Overflow case - should saturate
        let result = saturating_time_acceleration(i64::MAX / 2, 10.0);
        assert!(result <= i64::MAX / 2);
    }
}
