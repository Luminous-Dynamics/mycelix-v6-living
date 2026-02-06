//! # Field Interference Service [Primitive 6]
//!
//! Wave optics on K-Vector fields. Computes pairwise and group interference
//! patterns to identify constructive (synergistic) and destructive (antagonistic)
//! agent relationships.
//!
//! Capabilities:
//! - Compute pairwise interference between two agents.
//! - Compute group interference for an arbitrary set of agents.
//! - Find all constructive pairs above a given amplitude threshold.
//! - Find all destructive pairs below a given amplitude threshold.
//! - Build a full network interference map (NxN matrix).
//!
//! ## Tier
//! Tier 3 (experimental). Requires `tier3-experimental` feature flag.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::{
    CyclePhase, Did, FieldInterference, FieldInterferenceDetectedEvent, Gate1Check, Gate2Warning,
    InterferenceType, KVectorSignature, LivingProtocolError, LivingProtocolEvent, LivingResult,
    OverallInterferenceType,
};

// =============================================================================
// GroupInterference
// =============================================================================

/// Aggregate interference pattern for a group of agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInterference {
    /// Number of agents in the group.
    pub agent_count: usize,
    /// Per-dimension mean amplitude across all pairwise interactions.
    pub mean_amplitude_per_dim: [f64; 8],
    /// Overall mean amplitude.
    pub mean_amplitude: f64,
    /// Fraction of pairwise interactions that are constructive.
    pub constructive_ratio: f64,
    /// Fraction of pairwise interactions that are destructive.
    pub destructive_ratio: f64,
    /// Fraction of pairwise interactions that are mixed.
    pub mixed_ratio: f64,
    /// Overall group interference type.
    pub overall_type: OverallInterferenceType,
    /// Computed at.
    pub computed_at: chrono::DateTime<Utc>,
}

// =============================================================================
// InterferenceMap
// =============================================================================

/// Full NxN interference map for a set of agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterferenceMap {
    /// Agent DIDs in row/column order.
    pub agents: Vec<Did>,
    /// Pairwise interference results, indexed as `[i * n + j]` for row i, col j.
    /// Only upper triangle is populated (symmetric); diagonal is None.
    pub matrix: Vec<Option<FieldInterference>>,
    /// Total number of agents.
    pub n: usize,
    /// Computed at.
    pub computed_at: chrono::DateTime<Utc>,
}

impl InterferenceMap {
    /// Get the interference between agent at index `i` and agent at index `j`.
    pub fn get(&self, i: usize, j: usize) -> Option<&FieldInterference> {
        if i == j || i >= self.n || j >= self.n {
            return None;
        }
        let (row, col) = if i < j { (i, j) } else { (j, i) };
        self.matrix[row * self.n + col].as_ref()
    }

    /// Get the interference between two agents by DID.
    pub fn get_by_did(&self, a: &Did, b: &Did) -> Option<&FieldInterference> {
        let i = self.agents.iter().position(|d| d == a)?;
        let j = self.agents.iter().position(|d| d == b)?;
        self.get(i, j)
    }
}

// =============================================================================
// FieldInterferenceService
// =============================================================================

/// Service for computing field interference patterns across the network.
#[derive(Debug)]
pub struct FieldInterferenceService {
    /// Events emitted since last drain.
    pending_events: Vec<LivingProtocolEvent>,
}

impl FieldInterferenceService {
    /// Create a new field interference service.
    pub fn new() -> Self {
        Self {
            pending_events: Vec::new(),
        }
    }

    /// Compute pairwise interference between two K-Vector signatures.
    /// Uses `FieldInterference::compute` from living-core.
    pub fn compute_pairwise(
        &mut self,
        a_did: &Did,
        a_kvec: &KVectorSignature,
        b_did: &Did,
        b_kvec: &KVectorSignature,
    ) -> FieldInterference {
        let mut interference = FieldInterference::compute(a_kvec, b_kvec);
        interference.agents = vec![a_did.clone(), b_did.clone()];

        let interference_type = match interference.overall_type {
            OverallInterferenceType::Constructive => InterferenceType::Constructive,
            OverallInterferenceType::Destructive => InterferenceType::Destructive,
            OverallInterferenceType::Mixed => InterferenceType::Mixed,
        };

        let event = FieldInterferenceDetectedEvent {
            agents: vec![a_did.clone(), b_did.clone()],
            interference_type,
            amplitude: interference.amplitude,
            timestamp: Utc::now(),
        };

        self.pending_events
            .push(LivingProtocolEvent::FieldInterferenceDetected(event));

        debug!(
            agent_a = %a_did,
            agent_b = %b_did,
            amplitude = interference.amplitude,
            overall = ?interference.overall_type,
            "Pairwise interference computed"
        );

        interference
    }

    /// Compute group interference for a set of agents.
    /// Performs all N*(N-1)/2 pairwise computations and aggregates.
    pub fn compute_group(
        &mut self,
        agents: &[(Did, KVectorSignature)],
    ) -> LivingResult<GroupInterference> {
        if agents.len() < 2 {
            return Err(LivingProtocolError::InsufficientFieldsForInterference);
        }

        let n = agents.len();
        let mut total_pairs = 0u64;
        let mut constructive_count = 0u64;
        let mut destructive_count = 0u64;
        let mut mixed_count = 0u64;
        let mut amplitude_sums = [0.0f64; 8];
        let mut total_amplitude = 0.0f64;

        for i in 0..n {
            for j in (i + 1)..n {
                let interference = FieldInterference::compute(&agents[i].1, &agents[j].1);

                match interference.overall_type {
                    OverallInterferenceType::Constructive => constructive_count += 1,
                    OverallInterferenceType::Destructive => destructive_count += 1,
                    OverallInterferenceType::Mixed => mixed_count += 1,
                }

                for (dim_idx, dim) in interference.pattern.iter().enumerate() {
                    amplitude_sums[dim_idx] += dim.amplitude;
                }

                total_amplitude += interference.amplitude;
                total_pairs += 1;
            }
        }

        let total_pairs_f = total_pairs as f64;
        let mean_amplitude_per_dim = {
            let mut result = [0.0; 8];
            for i in 0..8 {
                result[i] = amplitude_sums[i] / total_pairs_f;
            }
            result
        };

        let constructive_ratio = constructive_count as f64 / total_pairs_f;
        let destructive_ratio = destructive_count as f64 / total_pairs_f;
        let mixed_ratio = mixed_count as f64 / total_pairs_f;

        let overall_type = if constructive_ratio >= 0.6 {
            OverallInterferenceType::Constructive
        } else if destructive_ratio >= 0.6 {
            OverallInterferenceType::Destructive
        } else {
            OverallInterferenceType::Mixed
        };

        let group = GroupInterference {
            agent_count: n,
            mean_amplitude_per_dim,
            mean_amplitude: total_amplitude / total_pairs_f,
            constructive_ratio,
            destructive_ratio,
            mixed_ratio,
            overall_type,
            computed_at: Utc::now(),
        };

        info!(
            agent_count = n,
            constructive_ratio = constructive_ratio,
            mean_amplitude = group.mean_amplitude,
            "Group interference computed"
        );

        Ok(group)
    }

    /// Find all constructive pairs with amplitude above the given threshold.
    /// Returns `(agent_a_did, agent_b_did, amplitude)`.
    pub fn find_constructive_pairs(
        &mut self,
        agents: &[(Did, KVectorSignature)],
        min_amplitude: f64,
    ) -> Vec<(Did, Did, f64)> {
        let mut results = Vec::new();

        for i in 0..agents.len() {
            for j in (i + 1)..agents.len() {
                let interference = FieldInterference::compute(&agents[i].1, &agents[j].1);

                if interference.overall_type == OverallInterferenceType::Constructive
                    && interference.amplitude >= min_amplitude
                {
                    results.push((
                        agents[i].0.clone(),
                        agents[j].0.clone(),
                        interference.amplitude,
                    ));
                }
            }
        }

        // Sort by amplitude descending
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        debug!(
            pair_count = results.len(),
            min_amplitude = min_amplitude,
            "Found constructive pairs"
        );

        results
    }

    /// Find all destructive pairs with amplitude above the given threshold.
    /// Returns `(agent_a_did, agent_b_did, amplitude)`.
    pub fn find_destructive_pairs(
        &mut self,
        agents: &[(Did, KVectorSignature)],
        min_amplitude: f64,
    ) -> Vec<(Did, Did, f64)> {
        let mut results = Vec::new();

        for i in 0..agents.len() {
            for j in (i + 1)..agents.len() {
                let interference = FieldInterference::compute(&agents[i].1, &agents[j].1);

                if interference.overall_type == OverallInterferenceType::Destructive
                    && interference.amplitude >= min_amplitude
                {
                    results.push((
                        agents[i].0.clone(),
                        agents[j].0.clone(),
                        interference.amplitude,
                    ));
                }
            }
        }

        // Sort by amplitude descending
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        debug!(
            pair_count = results.len(),
            min_amplitude = min_amplitude,
            "Found destructive pairs"
        );

        results
    }

    /// Build the full NxN interference map for a set of agents.
    /// Only the upper triangle is computed (the matrix is symmetric).
    pub fn network_interference_map(
        &mut self,
        agents: &[(Did, KVectorSignature)],
    ) -> LivingResult<InterferenceMap> {
        if agents.len() < 2 {
            return Err(LivingProtocolError::InsufficientFieldsForInterference);
        }

        let n = agents.len();
        let mut matrix: Vec<Option<FieldInterference>> = vec![None; n * n];
        let dids: Vec<Did> = agents.iter().map(|(d, _)| d.clone()).collect();

        for i in 0..n {
            for j in (i + 1)..n {
                let mut interference = FieldInterference::compute(&agents[i].1, &agents[j].1);
                interference.agents = vec![agents[i].0.clone(), agents[j].0.clone()];
                matrix[i * n + j] = Some(interference);
            }
        }

        info!(
            agent_count = n,
            pairs_computed = n * (n - 1) / 2,
            "Network interference map computed"
        );

        Ok(InterferenceMap {
            agents: dids,
            matrix,
            n,
            computed_at: Utc::now(),
        })
    }

    /// Drain pending events.
    pub fn drain_events(&mut self) -> Vec<LivingProtocolEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

impl Default for FieldInterferenceService {
    fn default() -> Self {
        Self::new()
    }
}

impl LivingPrimitive for FieldInterferenceService {
    fn primitive_id(&self) -> &str {
        "field_interference"
    }

    fn primitive_number(&self) -> u8 {
        6
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Consciousness
    }

    fn tier(&self) -> u8 {
        3
    }

    fn on_phase_change(
        &mut self,
        _new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        Ok(self.drain_events())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        // Field interference is a stateless computation service.
        // Gate 1 checks are validated at computation time (e.g., minimum 2 agents).
        vec![Gate1Check {
            invariant: "field_interference_stateless".to_string(),
            passed: true,
            details: None,
        }]
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        // No persistent state to warn about.
        vec![]
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        // Active during Eros (attractor fields) and CoCreation (entanglement)
        matches!(
            phase,
            CyclePhase::Eros | CyclePhase::CoCreation | CyclePhase::EmergentPersonhood
        )
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "primitive": "field_interference",
            "pending_events": self.pending_events.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_agent(did: &str, values: [f64; 8]) -> (Did, KVectorSignature) {
        (
            did.to_string(),
            KVectorSignature::from_array(values, Utc::now()),
        )
    }

    #[test]
    fn test_pairwise_constructive() {
        let mut service = FieldInterferenceService::new();
        let a = make_agent("did:a", [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]);
        let b = make_agent("did:b", [0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6]);

        let interference = service.compute_pairwise(&a.0, &a.1, &b.0, &b.1);
        assert_eq!(
            interference.overall_type,
            OverallInterferenceType::Constructive
        );
        assert!(interference.amplitude > 0.0);
        assert_eq!(interference.agents.len(), 2);
    }

    #[test]
    fn test_pairwise_emits_event() {
        let mut service = FieldInterferenceService::new();
        let a = make_agent("did:a", [0.5; 8]);
        let b = make_agent("did:b", [0.5; 8]);

        service.compute_pairwise(&a.0, &a.1, &b.0, &b.1);

        let events = service.drain_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            LivingProtocolEvent::FieldInterferenceDetected(e) => {
                assert_eq!(e.agents.len(), 2);
            }
            _ => panic!("Expected FieldInterferenceDetected event"),
        }
    }

    #[test]
    fn test_group_interference_requires_minimum_agents() {
        let mut service = FieldInterferenceService::new();
        let agents = vec![make_agent("did:a", [0.5; 8])];

        let result = service.compute_group(&agents);
        assert!(result.is_err());
    }

    #[test]
    fn test_group_interference_all_constructive() {
        let mut service = FieldInterferenceService::new();
        let agents = vec![
            make_agent("did:a", [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
            make_agent("did:b", [0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6]),
            make_agent("did:c", [0.7, 0.7, 0.7, 0.7, 0.7, 0.7, 0.7, 0.7]),
        ];

        let group = service.compute_group(&agents).unwrap();
        assert_eq!(group.agent_count, 3);
        assert!(group.constructive_ratio > 0.0);
        assert!(group.mean_amplitude > 0.0);
    }

    #[test]
    fn test_find_constructive_pairs() {
        let mut service = FieldInterferenceService::new();
        let agents = vec![
            make_agent("did:a", [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
            make_agent("did:b", [0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6]),
            make_agent("did:c", [0.7, 0.7, 0.7, 0.7, 0.7, 0.7, 0.7, 0.7]),
        ];

        let pairs = service.find_constructive_pairs(&agents, 0.0);
        // All same-sign agents should be constructive
        assert!(!pairs.is_empty());
        // Sorted by amplitude descending
        if pairs.len() > 1 {
            assert!(pairs[0].2 >= pairs[1].2);
        }
    }

    #[test]
    fn test_find_destructive_pairs_none_when_all_positive() {
        let mut service = FieldInterferenceService::new();
        let agents = vec![make_agent("did:a", [0.5; 8]), make_agent("did:b", [0.6; 8])];

        let pairs = service.find_destructive_pairs(&agents, 0.0);
        // All positive values -> constructive, no destructive
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_network_interference_map() {
        let mut service = FieldInterferenceService::new();
        let agents = vec![
            make_agent("did:a", [0.5; 8]),
            make_agent("did:b", [0.6; 8]),
            make_agent("did:c", [0.7; 8]),
        ];

        let map = service.network_interference_map(&agents).unwrap();
        assert_eq!(map.n, 3);
        assert_eq!(map.agents.len(), 3);

        // Upper triangle should be populated
        assert!(map.get(0, 1).is_some());
        assert!(map.get(0, 2).is_some());
        assert!(map.get(1, 2).is_some());

        // Diagonal should be None
        assert!(map.get(0, 0).is_none());

        // Lower triangle accesses upper triangle (symmetric)
        assert!(map.get(1, 0).is_some());
    }

    #[test]
    fn test_interference_map_get_by_did() {
        let mut service = FieldInterferenceService::new();
        let agents = vec![make_agent("did:a", [0.5; 8]), make_agent("did:b", [0.6; 8])];

        let map = service.network_interference_map(&agents).unwrap();
        let result = map.get_by_did(&"did:a".to_string(), &"did:b".to_string());
        assert!(result.is_some());

        // Reverse order should also work
        let result_rev = map.get_by_did(&"did:b".to_string(), &"did:a".to_string());
        assert!(result_rev.is_some());
    }

    #[test]
    fn test_interference_map_requires_minimum_agents() {
        let mut service = FieldInterferenceService::new();
        let agents = vec![make_agent("did:a", [0.5; 8])];

        let result = service.network_interference_map(&agents);
        assert!(result.is_err());
    }

    #[test]
    fn test_living_primitive_trait() {
        let service = FieldInterferenceService::new();
        assert_eq!(service.primitive_id(), "field_interference");
        assert_eq!(service.primitive_number(), 6);
        assert_eq!(service.module(), PrimitiveModule::Consciousness);
        assert_eq!(service.tier(), 3);
        assert!(service.is_active_in_phase(CyclePhase::Eros));
        assert!(service.is_active_in_phase(CyclePhase::CoCreation));
        assert!(!service.is_active_in_phase(CyclePhase::Shadow));
    }

    #[test]
    fn test_group_interference_ratios_sum_to_one() {
        let mut service = FieldInterferenceService::new();
        let agents = vec![
            make_agent("did:a", [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
            make_agent("did:b", [0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6, 0.6]),
            make_agent("did:c", [0.7, 0.7, 0.7, 0.7, 0.7, 0.7, 0.7, 0.7]),
            make_agent("did:d", [0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3]),
        ];

        let group = service.compute_group(&agents).unwrap();
        let total = group.constructive_ratio + group.destructive_ratio + group.mixed_ratio;
        assert!(
            (total - 1.0).abs() < 1e-10,
            "Ratios should sum to 1.0, got {}",
            total
        );
    }
}
