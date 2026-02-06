//! # [12] Beauty as Validity
//!
//! Scores proposals on five aesthetic dimensions: symmetry, economy,
//! resonance, surprise, and completeness. Used during the Beauty phase
//! of the 28-day Metabolism Cycle.
//!
//! This is a Rust-native service (no Holochain zome) that provides
//! heuristic scoring via text analysis. Each dimension produces a
//! score in [0.0, 1.0], which are combined with weights:
//!
//! | Dimension     | Weight |
//! |---------------|--------|
//! | Symmetry      | 0.20   |
//! | Economy       | 0.20   |
//! | Resonance     | 0.25   |
//! | Surprise      | 0.15   |
//! | Completeness  | 0.20   |
//!
//! ## Constitutional Alignment
//!
//! Resonant Coherence (Harmony 1) — beauty is a proxy for deep structural
//! quality that purely quantitative metrics miss.
//!
//! ## Epistemic Classification: E1/N3/M1

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use living_core::{
    BeautyScore, BeautyScoredEvent, CyclePhase, Did, EpistemicClassification, EpistemicTier,
    Gate1Check, Gate2Warning, LivingPrimitive, LivingProtocolEvent, LivingResult, MaterialityTier,
    NormativeTier, PrimitiveModule,
};

// =============================================================================
// Constants
// =============================================================================

/// Default weight for symmetry.
pub const WEIGHT_SYMMETRY: f64 = 0.20;
/// Default weight for economy.
pub const WEIGHT_ECONOMY: f64 = 0.20;
/// Default weight for resonance.
pub const WEIGHT_RESONANCE: f64 = 0.25;
/// Default weight for surprise.
pub const WEIGHT_SURPRISE: f64 = 0.15;
/// Default weight for completeness.
pub const WEIGHT_COMPLETENESS: f64 = 0.20;

// =============================================================================
// Beauty Validity Engine
// =============================================================================

/// Engine that scores proposals on aesthetic dimensions.
///
/// Multiple scorers can score the same proposal. The engine aggregates
/// scores and determines whether a proposal meets the beauty threshold.
pub struct BeautyValidityEngine {
    /// Per-proposal scores from individual scorers.
    scored_proposals: HashMap<String, Vec<ScoredEntry>>,
}

/// An individual scorer's assessment of a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredEntry {
    pub scorer_did: Did,
    pub score: BeautyScore,
}

impl BeautyValidityEngine {
    /// Create a new Beauty Validity engine.
    pub fn new() -> Self {
        Self {
            scored_proposals: HashMap::new(),
        }
    }

    /// Score a proposal on all five aesthetic dimensions.
    ///
    /// # Arguments
    ///
    /// * `proposal_id` — Unique identifier for the proposal.
    /// * `proposal_content` — The textual content of the proposal.
    /// * `scorer_did` — DID of the agent performing the scoring.
    /// * `existing_patterns` — Existing patterns/proposals in the ecosystem
    ///   (used for resonance and surprise computation).
    /// * `requirements` — Requirements the proposal should address
    ///   (used for completeness computation).
    ///
    /// # Returns
    ///
    /// A `BeautyScoredEvent` recording the score.
    pub fn score_proposal(
        &mut self,
        proposal_id: &str,
        proposal_content: &str,
        scorer_did: &str,
        existing_patterns: &[String],
        requirements: &[String],
    ) -> BeautyScoredEvent {
        let symmetry = self.compute_symmetry(proposal_content);
        let economy = self.compute_economy(proposal_content);
        let resonance = self.compute_resonance(proposal_content, existing_patterns);
        let surprise = self.compute_surprise(proposal_content, existing_patterns);
        let completeness = self.compute_completeness(proposal_content, requirements);

        let score = BeautyScore::compute(symmetry, economy, resonance, surprise, completeness);

        let entry = ScoredEntry {
            scorer_did: scorer_did.to_string(),
            score: score.clone(),
        };

        self.scored_proposals
            .entry(proposal_id.to_string())
            .or_default()
            .push(entry);

        tracing::info!(
            proposal_id = proposal_id,
            scorer_did = scorer_did,
            composite = score.composite,
            "Proposal beauty score computed"
        );

        BeautyScoredEvent {
            proposal_id: proposal_id.to_string(),
            score,
            scorer_did: scorer_did.to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Compute the symmetry score: structural balance and proportion.
    ///
    /// Heuristics:
    /// - Measures the balance of content distribution across sections/paragraphs.
    /// - Checks for balanced use of punctuation (opening/closing pairs).
    /// - Rewards structural regularity.
    pub fn compute_symmetry(&self, content: &str) -> f64 {
        if content.is_empty() {
            return 0.0;
        }

        let paragraphs: Vec<&str> = content
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect();

        if paragraphs.len() <= 1 {
            // Single block: moderate symmetry.
            return 0.5;
        }

        // Measure length variance across paragraphs.
        let lengths: Vec<f64> = paragraphs.iter().map(|p| p.len() as f64).collect();
        let mean_len = lengths.iter().sum::<f64>() / lengths.len() as f64;
        let variance =
            lengths.iter().map(|l| (l - mean_len).powi(2)).sum::<f64>() / lengths.len() as f64;
        let std_dev = variance.sqrt();

        // Coefficient of variation: lower is more symmetric.
        let cv = if mean_len > 0.0 {
            std_dev / mean_len
        } else {
            1.0
        };

        // Also check bracket/paren balance.
        let open_parens = content.matches('(').count();
        let close_parens = content.matches(')').count();
        let open_brackets = content.matches('[').count();
        let close_brackets = content.matches(']').count();
        let open_braces = content.matches('{').count();
        let close_braces = content.matches('}').count();

        let paren_balance = if open_parens + close_parens > 0 {
            1.0 - ((open_parens as f64 - close_parens as f64).abs()
                / (open_parens + close_parens) as f64)
        } else {
            1.0
        };

        let bracket_balance = if open_brackets + close_brackets > 0 {
            1.0 - ((open_brackets as f64 - close_brackets as f64).abs()
                / (open_brackets + close_brackets) as f64)
        } else {
            1.0
        };

        let brace_balance = if open_braces + close_braces > 0 {
            1.0 - ((open_braces as f64 - close_braces as f64).abs()
                / (open_braces + close_braces) as f64)
        } else {
            1.0
        };

        let structural_balance = (paren_balance + bracket_balance + brace_balance) / 3.0;

        // Combine: low CV is good (more uniform paragraphs).
        let length_score = (1.0 - cv.min(1.0)).max(0.0);

        ((length_score * 0.6) + (structural_balance * 0.4)).clamp(0.0, 1.0)
    }

    /// Compute the economy score: minimal complexity for the goal.
    ///
    /// Heuristics:
    /// - Shorter content relative to information density is better.
    /// - Penalizes excessive repetition.
    /// - Rewards conciseness.
    pub fn compute_economy(&self, content: &str) -> f64 {
        if content.is_empty() {
            return 0.0;
        }

        let words: Vec<&str> = content.split_whitespace().collect();
        let word_count = words.len();

        if word_count == 0 {
            return 0.0;
        }

        // Unique word ratio: higher ratio = more diverse vocabulary = more economical.
        let unique_words: std::collections::HashSet<&str> = words
            .iter()
            .map(|w| w.to_lowercase().leak() as &str)
            .collect();
        let unique_ratio = unique_words.len() as f64 / word_count as f64;

        // Penalize very long proposals (diminishing returns after ~500 words).
        let length_penalty = if word_count > 500 {
            (500.0 / word_count as f64).sqrt()
        } else {
            1.0
        };

        // Penalize very short proposals (insufficient detail under ~50 words).
        let brevity_bonus = if word_count < 50 {
            word_count as f64 / 50.0
        } else {
            1.0
        };

        // Sentence-to-word ratio: moderate sentence length is economical.
        let sentence_count = content.matches(['.', '!', '?']).count().max(1);
        let avg_sentence_len = word_count as f64 / sentence_count as f64;

        // Ideal average sentence length: 15-25 words.
        let sentence_score = if (15.0..=25.0).contains(&avg_sentence_len) {
            1.0
        } else if avg_sentence_len < 15.0 {
            avg_sentence_len / 15.0
        } else {
            (50.0 - avg_sentence_len).max(0.0) / 25.0
        };

        let raw =
            unique_ratio * 0.4 + length_penalty * 0.2 + brevity_bonus * 0.2 + sentence_score * 0.2;
        raw.clamp(0.0, 1.0)
    }

    /// Compute the resonance score: alignment with existing patterns.
    ///
    /// Measures how well the proposal fits with the existing ecosystem
    /// by checking for shared vocabulary and thematic overlap.
    pub fn compute_resonance(&self, content: &str, existing_patterns: &[String]) -> f64 {
        if content.is_empty() || existing_patterns.is_empty() {
            return 0.5; // Neutral resonance when no context.
        }

        let content_words: std::collections::HashSet<String> = content
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|w| w.len() > 3) // Skip short words.
            .collect();

        if content_words.is_empty() {
            return 0.5;
        }

        let mut total_overlap = 0.0;

        for pattern in existing_patterns {
            let pattern_words: std::collections::HashSet<String> = pattern
                .split_whitespace()
                .map(|w| {
                    w.to_lowercase()
                        .trim_matches(|c: char| !c.is_alphanumeric())
                        .to_string()
                })
                .filter(|w| w.len() > 3)
                .collect();

            if pattern_words.is_empty() {
                continue;
            }

            let intersection = content_words.intersection(&pattern_words).count();
            let union = content_words.union(&pattern_words).count();

            if union > 0 {
                total_overlap += intersection as f64 / union as f64;
            }
        }

        let avg_overlap = total_overlap / existing_patterns.len() as f64;

        // Scale: moderate overlap is best (too much means no novelty,
        // too little means no resonance).
        // Optimal Jaccard overlap is around 0.2-0.4.
        let resonance = if avg_overlap <= 0.3 {
            avg_overlap / 0.3
        } else if avg_overlap <= 0.6 {
            1.0
        } else {
            1.0 - ((avg_overlap - 0.6) / 0.4).min(1.0) * 0.5
        };

        resonance.clamp(0.0, 1.0)
    }

    /// Compute the surprise score: novelty and unexpectedness.
    ///
    /// Measures how different the proposal is from existing patterns.
    /// Inversely related to resonance, but not a simple inverse.
    pub fn compute_surprise(&self, content: &str, existing_patterns: &[String]) -> f64 {
        if content.is_empty() {
            return 0.0;
        }

        if existing_patterns.is_empty() {
            return 0.8; // High surprise when nothing to compare against.
        }

        let content_words: std::collections::HashSet<String> = content
            .split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|w| w.len() > 3)
            .collect();

        if content_words.is_empty() {
            return 0.5;
        }

        // Count words that appear in content but NOT in any existing pattern.
        let all_existing_words: std::collections::HashSet<String> = existing_patterns
            .iter()
            .flat_map(|p| {
                p.split_whitespace()
                    .map(|w| {
                        w.to_lowercase()
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_string()
                    })
                    .filter(|w| w.len() > 3)
            })
            .collect();

        let novel_words: std::collections::HashSet<&String> = content_words
            .iter()
            .filter(|w| !all_existing_words.contains(*w))
            .collect();

        let novelty_ratio = novel_words.len() as f64 / content_words.len() as f64;

        // Moderate novelty is ideal: too much means incomprehensible,
        // too little means nothing new.
        let surprise = if novelty_ratio <= 0.5 {
            novelty_ratio * 2.0
        } else {
            1.0 - (novelty_ratio - 0.5) * 0.6
        };

        surprise.clamp(0.0, 1.0)
    }

    /// Compute the completeness score: coverage of requirements.
    ///
    /// Measures what fraction of the listed requirements are addressed
    /// by the proposal content.
    pub fn compute_completeness(&self, content: &str, requirements: &[String]) -> f64 {
        if requirements.is_empty() {
            return 1.0; // Vacuously complete.
        }

        if content.is_empty() {
            return 0.0;
        }

        let content_lower = content.to_lowercase();
        let mut addressed = 0;

        for req in requirements {
            // A requirement is considered addressed if any significant words
            // from it appear in the content.
            let req_words: Vec<String> = req
                .split_whitespace()
                .map(|w| {
                    w.to_lowercase()
                        .trim_matches(|c: char| !c.is_alphanumeric())
                        .to_string()
                })
                .filter(|w| w.len() > 3)
                .collect();

            if req_words.is_empty() {
                addressed += 1; // Trivial requirement.
                continue;
            }

            let found = req_words
                .iter()
                .filter(|w| content_lower.contains(w.as_str()))
                .count();

            // If at least half the significant words are present, consider addressed.
            if found as f64 >= (req_words.len() as f64 * 0.5) {
                addressed += 1;
            }
        }

        (addressed as f64 / requirements.len() as f64).clamp(0.0, 1.0)
    }

    /// Aggregate all scores for a proposal into a single composite.
    ///
    /// Returns the average of all individual scorer scores.
    pub fn aggregate_scores(&self, proposal_id: &str) -> Option<BeautyScore> {
        let entries = self.scored_proposals.get(proposal_id)?;
        if entries.is_empty() {
            return None;
        }

        let n = entries.len() as f64;
        let sum_sym: f64 = entries.iter().map(|e| e.score.symmetry).sum();
        let sum_eco: f64 = entries.iter().map(|e| e.score.economy).sum();
        let sum_res: f64 = entries.iter().map(|e| e.score.resonance).sum();
        let sum_sur: f64 = entries.iter().map(|e| e.score.surprise).sum();
        let sum_com: f64 = entries.iter().map(|e| e.score.completeness).sum();

        Some(BeautyScore::compute(
            sum_sym / n,
            sum_eco / n,
            sum_res / n,
            sum_sur / n,
            sum_com / n,
        ))
    }

    /// Whether a proposal meets the beauty threshold.
    ///
    /// Uses the aggregated score if multiple scorers have contributed.
    pub fn meets_threshold(&self, proposal_id: &str, threshold: f64) -> bool {
        self.aggregate_scores(proposal_id)
            .map(|s| s.composite >= threshold)
            .unwrap_or(false)
    }

    /// Get all scored proposals.
    pub fn get_scored_proposals(&self) -> &HashMap<String, Vec<ScoredEntry>> {
        &self.scored_proposals
    }

    /// Get the number of scored proposals.
    pub fn scored_count(&self) -> usize {
        self.scored_proposals.len()
    }

    /// Epistemic classification for Beauty as Validity.
    pub fn classification() -> EpistemicClassification {
        EpistemicClassification {
            e: EpistemicTier::Testimonial, // E1
            n: NormativeTier::Axiomatic,   // N3
            m: MaterialityTier::Temporal,  // M1
        }
    }
}

impl Default for BeautyValidityEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LivingPrimitive Implementation
// =============================================================================

impl LivingPrimitive for BeautyValidityEngine {
    fn primitive_id(&self) -> &str {
        "beauty_validity"
    }

    fn primitive_number(&self) -> u8 {
        12
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Epistemics
    }

    fn tier(&self) -> u8 {
        1 // Tier 1: always on
    }

    fn on_phase_change(&mut self, new_phase: CyclePhase) -> LivingResult<Vec<LivingProtocolEvent>> {
        if new_phase == CyclePhase::Beauty {
            tracing::info!(
                scored_proposals = self.scored_proposals.len(),
                "Entering Beauty phase"
            );
        }
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        // Gate 1: all score components must be in [0.0, 1.0].
        let all_bounded = self.scored_proposals.values().all(|entries| {
            entries.iter().all(|e| {
                let s = &e.score;
                (0.0..=1.0).contains(&s.symmetry)
                    && (0.0..=1.0).contains(&s.economy)
                    && (0.0..=1.0).contains(&s.resonance)
                    && (0.0..=1.0).contains(&s.surprise)
                    && (0.0..=1.0).contains(&s.completeness)
                    && (0.0..=1.0).contains(&s.composite)
            })
        });

        vec![Gate1Check {
            invariant: "beauty_scores_bounded_0_1".to_string(),
            passed: all_bounded,
            details: if !all_bounded {
                Some("One or more beauty score components are outside [0.0, 1.0]".into())
            } else {
                None
            },
        }]
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Warn if any proposals have very low beauty scores.
        for (proposal_id, entries) in &self.scored_proposals {
            if let Some(agg) = self.aggregate_scores(proposal_id) {
                if agg.composite < 0.2 {
                    warnings.push(Gate2Warning {
                        harmony_violated: "Resonant Coherence (Harmony 1)".to_string(),
                        severity: 0.3,
                        reputation_impact: 0.0,
                        reasoning: format!(
                            "Proposal {} has very low beauty score ({:.2}) — consider revision",
                            proposal_id, agg.composite
                        ),
                        user_may_proceed: true,
                    });
                }
            }

            // Warn if only one scorer has scored.
            if entries.len() == 1 {
                warnings.push(Gate2Warning {
                    harmony_violated: "Resonant Coherence (Harmony 1)".to_string(),
                    severity: 0.1,
                    reputation_impact: 0.0,
                    reasoning: format!(
                        "Proposal {} has only 1 beauty scorer — additional perspectives recommended",
                        proposal_id
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        phase == CyclePhase::Beauty
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let total_scores: usize = self.scored_proposals.values().map(|v| v.len()).sum();
        serde_json::json!({
            "scored_proposals": self.scored_proposals.len(),
            "total_individual_scores": total_scores,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_proposal_basic() {
        let mut engine = BeautyValidityEngine::new();
        let event = engine.score_proposal(
            "prop-1",
            "This is a well-balanced proposal that addresses governance reform.\n\nIt introduces new voting mechanisms that are efficient and transparent.",
            "did:scorer:alice",
            &[],
            &[],
        );

        assert_eq!(event.proposal_id, "prop-1");
        assert_eq!(event.scorer_did, "did:scorer:alice");
        assert!(event.score.composite >= 0.0);
        assert!(event.score.composite <= 1.0);
    }

    #[test]
    fn test_all_scores_bounded() {
        let mut engine = BeautyValidityEngine::new();
        let content = "A proposal about improving the consensus mechanism.\n\n\
            It addresses scalability, security, and decentralization.\n\n\
            The approach uses novel cryptographic techniques.";

        let event = engine.score_proposal(
            "prop-x",
            content,
            "did:scorer:bob",
            &["existing consensus mechanism uses proof of stake".to_string()],
            &["scalability".to_string(), "security".to_string()],
        );

        let s = &event.score;
        assert!(
            (0.0..=1.0).contains(&s.symmetry),
            "symmetry: {}",
            s.symmetry
        );
        assert!((0.0..=1.0).contains(&s.economy), "economy: {}", s.economy);
        assert!(
            (0.0..=1.0).contains(&s.resonance),
            "resonance: {}",
            s.resonance
        );
        assert!(
            (0.0..=1.0).contains(&s.surprise),
            "surprise: {}",
            s.surprise
        );
        assert!(
            (0.0..=1.0).contains(&s.completeness),
            "completeness: {}",
            s.completeness
        );
        assert!(
            (0.0..=1.0).contains(&s.composite),
            "composite: {}",
            s.composite
        );

        // Gate 1 should pass.
        let checks = engine.gate1_check();
        assert!(checks[0].passed, "Gate 1 should pass with bounded scores");
    }

    #[test]
    fn test_empty_content_scores() {
        let engine = BeautyValidityEngine::new();
        assert_eq!(engine.compute_symmetry(""), 0.0);
        assert_eq!(engine.compute_economy(""), 0.0);
        assert_eq!(engine.compute_surprise("", &[]), 0.0);
        assert_eq!(engine.compute_completeness("", &["req".to_string()]), 0.0);
    }

    #[test]
    fn test_symmetry_balanced_paragraphs() {
        let engine = BeautyValidityEngine::new();

        // Balanced paragraphs.
        let balanced = "This is the first part of the proposal.\n\n\
            This is the second part of the proposal.\n\n\
            This is the third part of the proposal.";

        // Imbalanced paragraphs.
        let imbalanced = "Short.\n\n\
            This is an extremely long second paragraph that goes on and on and on about many different things and never seems to end because it just keeps talking about more and more topics without any real structure or organization whatsoever.";

        let balanced_score = engine.compute_symmetry(balanced);
        let imbalanced_score = engine.compute_symmetry(imbalanced);

        assert!(
            balanced_score > imbalanced_score,
            "Balanced ({}) should score higher than imbalanced ({})",
            balanced_score,
            imbalanced_score
        );
    }

    #[test]
    fn test_symmetry_bracket_balance() {
        let engine = BeautyValidityEngine::new();

        let balanced = "Function f(x) returns (y). Array [a, b] maps to [c, d].";
        let unbalanced = "Function f(x returns y. Array [a, b maps to c, d.";

        let balanced_score = engine.compute_symmetry(balanced);
        let unbalanced_score = engine.compute_symmetry(unbalanced);

        assert!(
            balanced_score >= unbalanced_score,
            "Balanced brackets ({}) should score >= unbalanced ({})",
            balanced_score,
            unbalanced_score
        );
    }

    #[test]
    fn test_economy_concise_vs_verbose() {
        let engine = BeautyValidityEngine::new();

        let concise = "The protocol improves throughput via parallel validation. \
            Each validator processes independent shards. \
            Results merge deterministically.";

        // Very repetitive text.
        let verbose = "The the the the protocol protocol protocol improves improves \
            throughput throughput throughput via via via parallel parallel parallel \
            validation validation validation. The the the the protocol protocol.";

        let concise_score = engine.compute_economy(concise);
        let verbose_score = engine.compute_economy(verbose);

        assert!(
            concise_score > verbose_score,
            "Concise ({}) should score higher than verbose ({})",
            concise_score,
            verbose_score
        );
    }

    #[test]
    fn test_resonance_with_existing_patterns() {
        let engine = BeautyValidityEngine::new();

        let existing = vec![
            "governance reform voting mechanisms transparency".to_string(),
            "decentralized consensus protocol validation".to_string(),
        ];

        let resonant = "This proposal improves governance voting mechanisms \
            for better transparency in the protocol.";
        let non_resonant = "Quantum entanglement photosynthesis metamorphosis \
            crystallography ontological epistemology.";

        let res_score = engine.compute_resonance(resonant, &existing);
        let non_res_score = engine.compute_resonance(non_resonant, &existing);

        assert!(
            res_score > non_res_score,
            "Resonant ({}) should score higher than non-resonant ({})",
            res_score,
            non_res_score
        );
    }

    #[test]
    fn test_resonance_empty_patterns() {
        let engine = BeautyValidityEngine::new();
        let score = engine.compute_resonance("some content", &[]);
        assert_eq!(score, 0.5, "No patterns => neutral resonance");
    }

    #[test]
    fn test_surprise_novel_content() {
        let engine = BeautyValidityEngine::new();

        let existing = vec!["governance reform voting mechanisms transparency".to_string()];

        let novel = "Introducing quantum-resistant lattice-based cryptographic \
            signatures for post-quantum decentralized identity verification.";
        let mundane = "governance reform voting mechanisms transparency improvements.";

        let novel_score = engine.compute_surprise(novel, &existing);
        let mundane_score = engine.compute_surprise(mundane, &existing);

        assert!(
            novel_score > mundane_score,
            "Novel ({}) should score higher than mundane ({})",
            novel_score,
            mundane_score
        );
    }

    #[test]
    fn test_surprise_no_patterns() {
        let engine = BeautyValidityEngine::new();
        let score = engine.compute_surprise("some content", &[]);
        assert_eq!(score, 0.8, "No patterns => high surprise");
    }

    #[test]
    fn test_completeness_all_requirements_met() {
        let engine = BeautyValidityEngine::new();

        let requirements = vec![
            "scalability improvements".to_string(),
            "security enhancements".to_string(),
            "decentralization guarantees".to_string(),
        ];

        let content = "This proposal provides scalability improvements through sharding. \
            Security enhancements are achieved via multi-sig validation. \
            Decentralization guarantees are ensured by distributed validators.";

        let score = engine.compute_completeness(content, &requirements);
        assert!(
            score > 0.8,
            "All requirements met should score high: {}",
            score
        );
    }

    #[test]
    fn test_completeness_no_requirements_met() {
        let engine = BeautyValidityEngine::new();

        let requirements = vec![
            "scalability improvements".to_string(),
            "security enhancements".to_string(),
        ];

        let content = "This is about gardening and cooking recipes.";

        let score = engine.compute_completeness(content, &requirements);
        assert!(
            score < 0.5,
            "No requirements met should score low: {}",
            score
        );
    }

    #[test]
    fn test_completeness_no_requirements() {
        let engine = BeautyValidityEngine::new();
        let score = engine.compute_completeness("anything", &[]);
        assert_eq!(score, 1.0, "No requirements => vacuously complete");
    }

    #[test]
    fn test_aggregate_scores_multiple_scorers() {
        let mut engine = BeautyValidityEngine::new();

        let content = "A balanced proposal.\n\nWith two sections.";
        engine.score_proposal("prop-1", content, "did:scorer:a", &[], &[]);
        engine.score_proposal("prop-1", content, "did:scorer:b", &[], &[]);
        engine.score_proposal("prop-1", content, "did:scorer:c", &[], &[]);

        let agg = engine
            .aggregate_scores("prop-1")
            .expect("Should have scores");
        assert!((0.0..=1.0).contains(&agg.composite));
    }

    #[test]
    fn test_aggregate_scores_nonexistent_proposal() {
        let engine = BeautyValidityEngine::new();
        assert!(engine.aggregate_scores("nonexistent").is_none());
    }

    #[test]
    fn test_meets_threshold() {
        let mut engine = BeautyValidityEngine::new();

        let content = "A well-structured proposal about governance reform.\n\n\
            It addresses all key concerns with clarity and precision.\n\n\
            The implementation plan is detailed and realistic.";

        engine.score_proposal("prop-good", content, "did:scorer:a", &[], &[]);

        // The threshold will depend on what the scoring produces.
        let agg = engine.aggregate_scores("prop-good").unwrap();
        assert!(engine.meets_threshold("prop-good", agg.composite - 0.01));
        assert!(!engine.meets_threshold("nonexistent", 0.1));
    }

    #[test]
    fn test_composite_weight_formula() {
        // Verify that BeautyScore::compute uses the correct weights.
        let score = BeautyScore::compute(1.0, 1.0, 1.0, 1.0, 1.0);
        let expected = 0.20 + 0.20 + 0.25 + 0.15 + 0.20; // = 1.0
        assert!(
            (score.composite - expected).abs() < 1e-10,
            "All 1.0 inputs should yield composite = 1.0, got {}",
            score.composite
        );

        let score_zero = BeautyScore::compute(0.0, 0.0, 0.0, 0.0, 0.0);
        assert!(
            score_zero.composite.abs() < 1e-10,
            "All 0.0 inputs should yield composite = 0.0, got {}",
            score_zero.composite
        );
    }

    #[test]
    fn test_composite_weight_individual() {
        // Test that each weight is applied correctly.
        let sym_only = BeautyScore::compute(1.0, 0.0, 0.0, 0.0, 0.0);
        assert!((sym_only.composite - 0.20).abs() < 1e-10);

        let eco_only = BeautyScore::compute(0.0, 1.0, 0.0, 0.0, 0.0);
        assert!((eco_only.composite - 0.20).abs() < 1e-10);

        let res_only = BeautyScore::compute(0.0, 0.0, 1.0, 0.0, 0.0);
        assert!((res_only.composite - 0.25).abs() < 1e-10);

        let sur_only = BeautyScore::compute(0.0, 0.0, 0.0, 1.0, 0.0);
        assert!((sur_only.composite - 0.15).abs() < 1e-10);

        let com_only = BeautyScore::compute(0.0, 0.0, 0.0, 0.0, 1.0);
        assert!((com_only.composite - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_classification() {
        let class = BeautyValidityEngine::classification();
        assert_eq!(class.e, EpistemicTier::Testimonial);
        assert_eq!(class.n, NormativeTier::Axiomatic);
        assert_eq!(class.m, MaterialityTier::Temporal);
    }

    #[test]
    fn test_active_in_beauty_phase_only() {
        let engine = BeautyValidityEngine::new();
        assert!(engine.is_active_in_phase(CyclePhase::Beauty));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
        assert!(!engine.is_active_in_phase(CyclePhase::NegativeCapability));
        assert!(!engine.is_active_in_phase(CyclePhase::CoCreation));
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = BeautyValidityEngine::new();
        assert_eq!(engine.primitive_id(), "beauty_validity");
        assert_eq!(engine.primitive_number(), 12);
        assert_eq!(engine.module(), PrimitiveModule::Epistemics);
        assert_eq!(engine.tier(), 1);
    }

    #[test]
    fn test_gate1_passes_with_valid_scores() {
        let mut engine = BeautyValidityEngine::new();
        engine.score_proposal("p1", "Test content.", "did:s", &[], &[]);
        let checks = engine.gate1_check();
        assert!(checks[0].passed);
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = BeautyValidityEngine::new();
        engine.score_proposal("p1", "Content.", "did:a", &[], &[]);
        engine.score_proposal("p1", "Content.", "did:b", &[], &[]);
        engine.score_proposal("p2", "Other.", "did:a", &[], &[]);

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["scored_proposals"], 2);
        assert_eq!(metrics["total_individual_scores"], 3);
    }

    #[test]
    fn test_single_paragraph_symmetry() {
        let engine = BeautyValidityEngine::new();
        let score = engine.compute_symmetry("Just one paragraph with no breaks.");
        assert_eq!(score, 0.5, "Single paragraph => moderate symmetry");
    }
}
