//! Fuzz target for beauty scoring.
//!
//! This target fuzzes the beauty validity engine to ensure:
//! - All scores remain bounded in [0, 1]
//! - No panics on arbitrary content
//! - Gate 1 invariants always pass
//!
//! ## Running
//!
//! ```bash
//! cargo +nightly fuzz run beauty_scoring -- -max_len=8192
//! ```

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use epistemics::beauty_validity::BeautyValidityEngine;
use libfuzzer_sys::fuzz_target;
use living_core::BeautyScore;

/// Beauty scoring operations to fuzz
#[derive(Debug, Arbitrary)]
enum BeautyOp {
    /// Score a proposal with arbitrary content
    ScoreProposal,
    /// Compute symmetry
    ComputeSymmetry,
    /// Compute economy
    ComputeEconomy,
    /// Compute resonance
    ComputeResonance,
    /// Compute surprise
    ComputeSurprise,
    /// Compute completeness
    ComputeCompleteness,
    /// Get aggregate scores
    AggregateScores,
    /// Check threshold
    CheckThreshold { threshold_u8: u8 },
}

fn extract_string(u: &mut Unstructured, max_len: usize) -> Result<String, arbitrary::Error> {
    let len: usize = u.int_in_range(0..=max_len)?;
    let bytes: Vec<u8> = (0..len)
        .map(|_| u.int_in_range(32u8..=126u8))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn extract_strings(u: &mut Unstructured, count: usize, max_len: usize) -> Result<Vec<String>, arbitrary::Error> {
    (0..count)
        .map(|_| extract_string(u, max_len))
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    let mut engine = BeautyValidityEngine::new();

    // Extract content for scoring
    let content = match extract_string(&mut u, 2000) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Extract patterns
    let pattern_count: usize = match u.int_in_range(0..=5) {
        Ok(n) => n,
        Err(_) => return,
    };
    let patterns = match extract_strings(&mut u, pattern_count, 200) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Extract requirements
    let req_count: usize = match u.int_in_range(0..=5) {
        Ok(n) => n,
        Err(_) => return,
    };
    let requirements = match extract_strings(&mut u, req_count, 50) {
        Ok(r) => r,
        Err(_) => return,
    };

    // Get number of operations
    let op_count: usize = match u.int_in_range(1..=20) {
        Ok(n) => n,
        Err(_) => return,
    };

    let mut proposal_id = 0;

    for _ in 0..op_count {
        let op: BeautyOp = match u.arbitrary() {
            Ok(op) => op,
            Err(_) => break,
        };

        match op {
            BeautyOp::ScoreProposal => {
                let id = format!("prop-{}", proposal_id);
                proposal_id += 1;

                let event = engine.score_proposal(
                    &id,
                    &content,
                    "did:scorer:fuzz",
                    &patterns,
                    &requirements,
                );

                // Invariant checks
                let s = &event.score;
                assert!(
                    s.symmetry >= 0.0 && s.symmetry <= 1.0,
                    "Symmetry {} out of bounds",
                    s.symmetry
                );
                assert!(
                    s.economy >= 0.0 && s.economy <= 1.0,
                    "Economy {} out of bounds",
                    s.economy
                );
                assert!(
                    s.resonance >= 0.0 && s.resonance <= 1.0,
                    "Resonance {} out of bounds",
                    s.resonance
                );
                assert!(
                    s.surprise >= 0.0 && s.surprise <= 1.0,
                    "Surprise {} out of bounds",
                    s.surprise
                );
                assert!(
                    s.completeness >= 0.0 && s.completeness <= 1.0,
                    "Completeness {} out of bounds",
                    s.completeness
                );
                assert!(
                    s.composite >= 0.0 && s.composite <= 1.0,
                    "Composite {} out of bounds",
                    s.composite
                );
            }
            BeautyOp::ComputeSymmetry => {
                let score = engine.compute_symmetry(&content);
                assert!(
                    score >= 0.0 && score <= 1.0,
                    "Symmetry {} out of bounds",
                    score
                );
            }
            BeautyOp::ComputeEconomy => {
                let score = engine.compute_economy(&content);
                assert!(
                    score >= 0.0 && score <= 1.0,
                    "Economy {} out of bounds",
                    score
                );
            }
            BeautyOp::ComputeResonance => {
                let score = engine.compute_resonance(&content, &patterns);
                assert!(
                    score >= 0.0 && score <= 1.0,
                    "Resonance {} out of bounds",
                    score
                );
            }
            BeautyOp::ComputeSurprise => {
                let score = engine.compute_surprise(&content, &patterns);
                assert!(
                    score >= 0.0 && score <= 1.0,
                    "Surprise {} out of bounds",
                    score
                );
            }
            BeautyOp::ComputeCompleteness => {
                let score = engine.compute_completeness(&content, &requirements);
                assert!(
                    score >= 0.0 && score <= 1.0,
                    "Completeness {} out of bounds",
                    score
                );
            }
            BeautyOp::AggregateScores => {
                if proposal_id > 0 {
                    let id = format!("prop-{}", proposal_id - 1);
                    if let Some(agg) = engine.aggregate_scores(&id) {
                        assert!(
                            agg.composite >= 0.0 && agg.composite <= 1.0,
                            "Aggregate composite {} out of bounds",
                            agg.composite
                        );
                    }
                }
            }
            BeautyOp::CheckThreshold { threshold_u8 } => {
                if proposal_id > 0 {
                    let id = format!("prop-{}", proposal_id - 1);
                    let threshold = (threshold_u8 as f64) / 255.0;
                    let _meets = engine.meets_threshold(&id, threshold);
                }
            }
        }
    }

    // Check Gate 1 at the end
    let checks = engine.gate1_check();
    for check in &checks {
        assert!(check.passed, "Gate 1 failed: {} - {:?}", check.invariant, check.details);
    }
});

/// Also fuzz BeautyScore::compute directly
#[cfg(feature = "direct")]
fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    // Get 5 values in [0, 1]
    let values: [u16; 5] = match u.arbitrary() {
        Ok(v) => v,
        Err(_) => return,
    };

    let vals: [f64; 5] = [
        (values[0] as f64) / 65535.0,
        (values[1] as f64) / 65535.0,
        (values[2] as f64) / 65535.0,
        (values[3] as f64) / 65535.0,
        (values[4] as f64) / 65535.0,
    ];

    let score = BeautyScore::compute(vals[0], vals[1], vals[2], vals[3], vals[4]);

    assert!(
        score.composite >= 0.0 && score.composite <= 1.0,
        "Composite {} out of bounds for inputs {:?}",
        score.composite,
        vals
    );
});
