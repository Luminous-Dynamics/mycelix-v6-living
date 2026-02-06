//! Fuzz target for wound healing state machine.
//!
//! This target fuzzes the wound healing engine to ensure:
//! - Phase transitions are forward-only
//! - Scar tissue strength > 1.0
//! - Gate 1 invariants always pass
//! - No panics under arbitrary operation sequences
//!
//! ## Running
//!
//! ```bash
//! cargo +nightly fuzz run wound_healing -- -max_len=1024
//! ```

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use living_core::traits::LivingPrimitive;
use living_core::{EventBus, InMemoryEventBus, WoundHealingConfig, WoundPhase, WoundSeverity};
use metabolism::WoundHealingEngine;
use std::sync::Arc;

/// Wound healing operations to fuzz
#[derive(Debug, Arbitrary)]
enum WoundOp {
    /// Create a new wound
    CreateWound { severity_idx: u8 },
    /// Advance phase for last wound
    AdvancePhase,
    /// Submit restitution
    SubmitRestitution,
    /// Form scar tissue
    FormScarTissue,
    /// Get wound for agent
    GetWoundsForAgent,
    /// Get active wounds
    GetActiveWounds,
    /// Heal wound fully
    HealFully,
    /// Gate 1 check
    Gate1Check,
    /// Gate 2 check
    Gate2Check,
}

fn idx_to_severity(idx: u8) -> WoundSeverity {
    match idx % 4 {
        0 => WoundSeverity::Minor,
        1 => WoundSeverity::Moderate,
        2 => WoundSeverity::Severe,
        _ => WoundSeverity::Critical,
    }
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
    let mut engine = WoundHealingEngine::new(WoundHealingConfig::default(), event_bus);

    // Track created wound IDs
    let mut wound_ids: Vec<String> = Vec::new();
    let mut agent_counter = 0u32;

    // Get number of operations
    let op_count: usize = match u.int_in_range(1..=50) {
        Ok(n) => n,
        Err(_) => return,
    };

    for _ in 0..op_count {
        let op: WoundOp = match u.arbitrary() {
            Ok(op) => op,
            Err(_) => break,
        };

        match op {
            WoundOp::CreateWound { severity_idx } => {
                let severity = idx_to_severity(severity_idx);
                let agent_did = format!("did:agent:{}", agent_counter);
                agent_counter = agent_counter.wrapping_add(1);

                if let Ok(wound) =
                    engine.create_wound(agent_did, severity, "fuzz cause".to_string())
                {
                    wound_ids.push(wound.id.clone());

                    // Invariant: new wound starts in Hemostasis
                    assert_eq!(
                        wound.phase,
                        WoundPhase::Hemostasis,
                        "New wound should start in Hemostasis"
                    );

                    // Invariant: restitution is present
                    assert!(
                        wound.restitution_required.is_some(),
                        "New wound should have restitution"
                    );
                }
            }
            WoundOp::AdvancePhase => {
                if let Some(wound_id) = wound_ids.last() {
                    // Get phase before
                    let phase_before = engine.get_wound(wound_id).map(|w| w.phase);

                    let result = engine.advance_phase(wound_id);

                    // If successful, verify forward-only
                    if let (Ok(new_phase), Some(old_phase)) = (result, phase_before) {
                        assert!(
                            old_phase.can_transition_to(&new_phase),
                            "Invalid transition: {:?} -> {:?}",
                            old_phase,
                            new_phase
                        );
                    }
                }
            }
            WoundOp::SubmitRestitution => {
                if let Some(wound_id) = wound_ids.last() {
                    use metabolism::wound_healing::RestitutionAction;

                    let actions = vec![RestitutionAction {
                        description: "Fuzz restitution".to_string(),
                        evidence: None,
                        completed_at: chrono::Utc::now(),
                        tx_hash: None,
                    }];

                    let _ = engine.submit_restitution(wound_id, actions);
                }
            }
            WoundOp::FormScarTissue => {
                if let Some(wound_id) = wound_ids.last() {
                    if let Ok(scar) = engine.form_scar_tissue(wound_id) {
                        // Invariant: scar tissue strength > 1.0
                        assert!(
                            scar.strength_multiplier > 1.0,
                            "Scar strength {} should be > 1.0",
                            scar.strength_multiplier
                        );
                    }
                }
            }
            WoundOp::GetWoundsForAgent => {
                if agent_counter > 0 {
                    let agent_did = format!("did:agent:{}", (agent_counter - 1) % 10);
                    let _wounds = engine.get_wounds_for_agent(&agent_did);
                }
            }
            WoundOp::GetActiveWounds => {
                let active = engine.get_active_wounds();
                // All active wounds should not be Healed
                for wound in &active {
                    assert_ne!(
                        wound.phase,
                        WoundPhase::Healed,
                        "Active wound should not be Healed"
                    );
                }
            }
            WoundOp::HealFully => {
                if let Some(wound_id) = wound_ids.last().cloned() {
                    if let Ok(healed) = engine.heal_fully(&wound_id) {
                        // Invariant: fully healed wound is in Healed phase
                        assert_eq!(
                            healed.phase,
                            WoundPhase::Healed,
                            "heal_fully should result in Healed phase"
                        );

                        // Invariant: has scar tissue
                        assert!(
                            healed.scar_tissue.is_some(),
                            "Fully healed wound should have scar tissue"
                        );

                        if let Some(scar) = &healed.scar_tissue {
                            assert!(
                                scar.strength_multiplier > 1.0,
                                "Scar strength should be > 1.0"
                            );
                        }
                    }
                }
            }
            WoundOp::Gate1Check => {
                let checks = engine.gate1_check();
                for check in &checks {
                    assert!(
                        check.passed,
                        "Gate 1 failed: {} - {:?}",
                        check.invariant,
                        check.details
                    );
                }
            }
            WoundOp::Gate2Check => {
                // Gate 2 warnings are informational, just ensure no panic
                let _warnings = engine.gate2_check();
            }
        }
    }

    // Final Gate 1 check
    let checks = engine.gate1_check();
    for check in &checks {
        assert!(
            check.passed,
            "Final Gate 1 failed: {} - {:?}",
            check.invariant,
            check.details
        );
    }

    // Verify all wound phase histories are monotonic
    for wound_id in &wound_ids {
        if let Some(wound) = engine.get_wound(wound_id) {
            for window in wound.phase_history.windows(2) {
                let from = &window[0].0;
                let to = &window[1].0;
                assert!(
                    from.can_transition_to(to),
                    "Invalid transition in history: {:?} -> {:?}",
                    from,
                    to
                );
            }
        }
    }
});
