//! # Temporal K-Vector Service [Primitive 5]
//!
//! Manages `TemporalKVector` instances per agent, tracking first and second
//! derivatives (velocity, acceleration) of K-Vector signatures over time.
//!
//! Capabilities:
//! - Register agents and bootstrap their temporal tracking.
//! - Update observations and emit `TemporalKVectorUpdated` events.
//! - Query per-agent velocity and acceleration.
//! - Detect anomalous changes across the population.
//! - Predict future K-Vector values for all tracked agents.
//!
//! ## Tier
//! Tier 1 (always on). No feature gate required.

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use living_core::{
    Did, KVectorSignature, TemporalKVector,
    TemporalKVectorUpdatedEvent,
    LivingProtocolEvent,
    Gate1Check, Gate2Warning,
    CyclePhase,
    LivingProtocolError, LivingResult,
};
use living_core::traits::{LivingPrimitive, PrimitiveModule};

/// Default maximum history length per agent.
const DEFAULT_MAX_HISTORY: usize = 365;

/// Default anomaly threshold (rate-of-change per day per dimension).
const DEFAULT_ANOMALY_THRESHOLD: f64 = 0.5;

/// Service managing temporal K-Vector tracking for all registered agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalKVectorService {
    /// Per-agent temporal K-Vector state.
    agents: HashMap<Did, TemporalKVector>,
    /// Maximum history entries to keep per agent.
    max_history: usize,
    /// Default anomaly threshold.
    anomaly_threshold: f64,
    /// Events emitted since last drain.
    #[serde(skip)]
    pending_events: Vec<LivingProtocolEvent>,
}

impl TemporalKVectorService {
    /// Create a new service with default configuration.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            max_history: DEFAULT_MAX_HISTORY,
            anomaly_threshold: DEFAULT_ANOMALY_THRESHOLD,
            pending_events: Vec::new(),
        }
    }

    /// Create a new service with custom history depth and anomaly threshold.
    pub fn with_config(max_history: usize, anomaly_threshold: f64) -> Self {
        Self {
            agents: HashMap::new(),
            max_history,
            anomaly_threshold,
            pending_events: Vec::new(),
        }
    }

    /// Register a new agent with an initial K-Vector observation.
    /// Returns the newly created `TemporalKVector`.
    pub fn register_agent(&mut self, did: &Did, initial_kvec: KVectorSignature) -> TemporalKVector {
        let temporal = TemporalKVector::new(initial_kvec, self.max_history);
        self.agents.insert(did.clone(), temporal.clone());
        info!(agent = %did, "Registered agent for temporal K-Vector tracking");
        temporal
    }

    /// Update an agent's K-Vector with a new observation.
    /// Computes derivatives and emits a `TemporalKVectorUpdated` event.
    ///
    /// Returns the event on success, or an error if the agent is not registered.
    pub fn update_observation(
        &mut self,
        did: &Did,
        kvec: KVectorSignature,
    ) -> LivingResult<TemporalKVectorUpdatedEvent> {
        let temporal = self
            .agents
            .get_mut(did)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(did.clone()))?;

        temporal.update(kvec);

        let rate = temporal.rate_of_change();
        let event = TemporalKVectorUpdatedEvent {
            agent_did: did.clone(),
            derivatives: temporal.velocity.to_vec(),
            rate_of_change: rate,
            timestamp: Utc::now(),
        };

        if temporal.has_anomalous_change(self.anomaly_threshold) {
            warn!(
                agent = %did,
                rate_of_change = %rate,
                "Anomalous K-Vector change detected"
            );
        }

        debug!(
            agent = %did,
            rate_of_change = %rate,
            "Temporal K-Vector updated"
        );

        self.pending_events
            .push(LivingProtocolEvent::TemporalKVectorUpdated(event.clone()));

        Ok(event)
    }

    /// Get the velocity (first derivative) for an agent.
    /// Returns `[0.0; 8]` semantics: rate of change per day for each K dimension.
    pub fn get_velocity(&self, did: &Did) -> LivingResult<[f64; 8]> {
        let temporal = self
            .agents
            .get(did)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(did.clone()))?;
        Ok(temporal.velocity)
    }

    /// Get the acceleration (second derivative) for an agent.
    pub fn get_acceleration(&self, did: &Did) -> LivingResult<[f64; 8]> {
        let temporal = self
            .agents
            .get(did)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(did.clone()))?;
        Ok(temporal.acceleration)
    }

    /// Get the overall rate of change for an agent.
    pub fn get_rate_of_change(&self, did: &Did) -> LivingResult<f64> {
        let temporal = self
            .agents
            .get(did)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(did.clone()))?;
        Ok(temporal.rate_of_change())
    }

    /// Detect agents with anomalous rate-of-change above the given threshold.
    /// Returns a list of `(Did, Vec<dimension_indices>)` where each dimension
    /// index indicates which K-Vector dimension is changing anomalously.
    pub fn detect_anomalies(&self, threshold: f64) -> Vec<(Did, Vec<usize>)> {
        let mut anomalies = Vec::new();

        for (did, temporal) in &self.agents {
            let anomalous_dims: Vec<usize> = temporal
                .velocity
                .iter()
                .enumerate()
                .filter(|(_, v)| v.abs() > threshold)
                .map(|(i, _)| i)
                .collect();

            if !anomalous_dims.is_empty() {
                anomalies.push((did.clone(), anomalous_dims));
            }
        }

        anomalies
    }

    /// Predict K-Vector values for all tracked agents at `days_forward` days
    /// from now. Uses the current velocity and acceleration to extrapolate.
    pub fn predict_all(&self, days_forward: f64) -> HashMap<Did, [f64; 8]> {
        self.agents
            .iter()
            .map(|(did, temporal)| (did.clone(), temporal.predict(days_forward)))
            .collect()
    }

    /// Predict K-Vector for a single agent.
    pub fn predict(&self, did: &Did, days_forward: f64) -> LivingResult<[f64; 8]> {
        let temporal = self
            .agents
            .get(did)
            .ok_or_else(|| LivingProtocolError::AgentNotFound(did.clone()))?;
        Ok(temporal.predict(days_forward))
    }

    /// Get the most volatile dimensions across all agents, ranked by
    /// aggregate absolute velocity.
    pub fn most_volatile_dimensions_global(&self, top_n: usize) -> Vec<(usize, f64)> {
        let mut dim_sums = [0.0f64; 8];
        let count = self.agents.len() as f64;

        if count == 0.0 {
            return Vec::new();
        }

        for temporal in self.agents.values() {
            for (i, v) in temporal.velocity.iter().enumerate() {
                dim_sums[i] += v.abs();
            }
        }

        // Average absolute velocity per dimension
        let mut indexed: Vec<(usize, f64)> = dim_sums
            .iter()
            .enumerate()
            .map(|(i, sum)| (i, sum / count))
            .collect();

        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        indexed.truncate(top_n);
        indexed
    }

    /// Get the temporal K-Vector state for a specific agent.
    pub fn get_temporal(&self, did: &Did) -> Option<&TemporalKVector> {
        self.agents.get(did)
    }

    /// Total number of tracked agents.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Drain pending events.
    pub fn drain_events(&mut self) -> Vec<LivingProtocolEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Check whether an agent is registered.
    pub fn is_registered(&self, did: &Did) -> bool {
        self.agents.contains_key(did)
    }

    /// Remove an agent from tracking.
    pub fn unregister_agent(&mut self, did: &Did) -> Option<TemporalKVector> {
        self.agents.remove(did)
    }
}

impl Default for TemporalKVectorService {
    fn default() -> Self {
        Self::new()
    }
}

impl LivingPrimitive for TemporalKVectorService {
    fn primitive_id(&self) -> &str {
        "temporal_k_vector"
    }

    fn primitive_number(&self) -> u8 {
        5
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Consciousness
    }

    fn tier(&self) -> u8 {
        1
    }

    fn on_phase_change(
        &mut self,
        _new_phase: CyclePhase,
    ) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Temporal K-Vector is always active regardless of phase.
        // Drain any pending events from recent updates.
        Ok(self.drain_events())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        // Gate 1: All velocity values must be finite
        for (did, temporal) in &self.agents {
            let all_finite = temporal
                .velocity
                .iter()
                .chain(temporal.acceleration.iter())
                .all(|v| v.is_finite());

            checks.push(Gate1Check {
                invariant: format!("temporal_kvec_finite:{}", did),
                passed: all_finite,
                details: if all_finite {
                    None
                } else {
                    Some(format!(
                        "Agent {} has non-finite derivative values",
                        did
                    ))
                },
            });
        }

        // Gate 1: Current K-Vector values must be in [0, 1]
        for (did, temporal) in &self.agents {
            let arr = temporal.current.as_array();
            let in_bounds = arr.iter().all(|v| *v >= 0.0 && *v <= 1.0);

            checks.push(Gate1Check {
                invariant: format!("kvec_bounded:{}", did),
                passed: in_bounds,
                details: if in_bounds {
                    None
                } else {
                    Some(format!(
                        "Agent {} has K-Vector values outside [0, 1]",
                        did
                    ))
                },
            });
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        // Warn if any agent has a high rate of change
        for (did, temporal) in &self.agents {
            let roc = temporal.rate_of_change();
            if roc > self.anomaly_threshold {
                warnings.push(Gate2Warning {
                    harmony_violated: "Continuous Evolution".to_string(),
                    severity: (roc / self.anomaly_threshold).min(1.0),
                    reputation_impact: -0.01 * roc,
                    reasoning: format!(
                        "Agent {} has rate-of-change {:.4} exceeding threshold {:.4}",
                        did, roc, self.anomaly_threshold
                    ),
                    user_may_proceed: true,
                });
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, _phase: CyclePhase) -> bool {
        // Always active
        true
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let total_agents = self.agents.len();
        let avg_roc: f64 = if total_agents > 0 {
            self.agents
                .values()
                .map(|t| t.rate_of_change())
                .sum::<f64>()
                / total_agents as f64
        } else {
            0.0
        };
        let anomalous_count = self.detect_anomalies(self.anomaly_threshold).len();

        serde_json::json!({
            "primitive": "temporal_k_vector",
            "tracked_agents": total_agents,
            "average_rate_of_change": avg_roc,
            "anomalous_agents": anomalous_count,
            "anomaly_threshold": self.anomaly_threshold,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration};

    fn make_kvec(values: [f64; 8], days_ago: i64) -> KVectorSignature {
        KVectorSignature::from_array(values, Utc::now() - Duration::days(days_ago))
    }

    fn make_kvec_at(values: [f64; 8], time: DateTime<Utc>) -> KVectorSignature {
        KVectorSignature::from_array(values, time)
    }

    #[test]
    fn test_register_agent() {
        let mut service = TemporalKVectorService::new();
        let did = "did:mycelix:agent1".to_string();
        let kvec = make_kvec([0.5; 8], 0);

        let temporal = service.register_agent(&did, kvec);
        assert_eq!(temporal.velocity, [0.0; 8]);
        assert_eq!(temporal.acceleration, [0.0; 8]);
        assert!(service.is_registered(&did));
        assert_eq!(service.agent_count(), 1);
    }

    #[test]
    fn test_update_observation_computes_velocity() {
        let mut service = TemporalKVectorService::new();
        let did = "did:mycelix:agent1".to_string();

        let initial = make_kvec([0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5], 1);
        service.register_agent(&did, initial);

        let updated = make_kvec([0.6, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5], 0);
        let event = service.update_observation(&did, updated).unwrap();

        // k_r changed by 0.1 in ~1 day => velocity[0] ~ 0.1
        assert!(event.derivatives[0] > 0.09 && event.derivatives[0] < 0.11);
        // Other dims unchanged
        assert!(event.derivatives[1].abs() < 0.01);
        assert!(event.rate_of_change > 0.0);
    }

    #[test]
    fn test_update_observation_agent_not_found() {
        let mut service = TemporalKVectorService::new();
        let kvec = make_kvec([0.5; 8], 0);
        let result = service.update_observation(&"nonexistent".to_string(), kvec);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_velocity_and_acceleration() {
        let mut service = TemporalKVectorService::new();
        let did = "did:mycelix:agent1".to_string();
        let base_time = Utc::now() - Duration::days(2);

        let initial = make_kvec_at([0.5; 8], base_time);
        service.register_agent(&did, initial);

        let update1 = make_kvec_at([0.6, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5], base_time + Duration::days(1));
        service.update_observation(&did, update1).unwrap();

        let vel = service.get_velocity(&did).unwrap();
        assert!(vel[0] > 0.09);

        let update2 = make_kvec_at([0.8, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5], base_time + Duration::days(2));
        service.update_observation(&did, update2).unwrap();

        let accel = service.get_acceleration(&did).unwrap();
        // Acceleration in dim 0 should be positive (velocity increased)
        assert!(accel[0] > 0.0);
    }

    #[test]
    fn test_detect_anomalies() {
        let mut service = TemporalKVectorService::new();

        // Agent with normal change
        let did_normal = "did:mycelix:normal".to_string();
        let initial = make_kvec([0.5; 8], 1);
        service.register_agent(&did_normal, initial);
        let update = make_kvec([0.51, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5], 0);
        service.update_observation(&did_normal, update).unwrap();

        // Agent with anomalous change
        let did_anomaly = "did:mycelix:anomaly".to_string();
        let initial = make_kvec([0.5; 8], 1);
        service.register_agent(&did_anomaly, initial);
        let update = make_kvec([0.99, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5], 0);
        service.update_observation(&did_anomaly, update).unwrap();

        let anomalies = service.detect_anomalies(0.3);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].0, did_anomaly);
        assert!(anomalies[0].1.contains(&0)); // dim 0 is anomalous
    }

    #[test]
    fn test_predict_all() {
        let mut service = TemporalKVectorService::new();

        let did = "did:mycelix:agent1".to_string();
        let base_time = Utc::now() - Duration::days(3);

        // Two observations to establish velocity WITHOUT acceleration:
        // t0: 0.5, t1: 0.55, t2: 0.6  -> velocity = 0.05/day, accel = 0
        let initial = make_kvec_at([0.5; 8], base_time);
        service.register_agent(&did, initial);

        let update1 = make_kvec_at(
            [0.55, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            base_time + Duration::days(1),
        );
        service.update_observation(&did, update1).unwrap();

        let update2 = make_kvec_at(
            [0.6, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            base_time + Duration::days(2),
        );
        service.update_observation(&did, update2).unwrap();

        // velocity[0] = 0.05/day, acceleration[0] = 0
        // predict(1 day) = 0.6 + 0.05*1 + 0 = 0.65
        let predictions = service.predict_all(1.0);
        let predicted = predictions.get(&did).unwrap();
        assert!(
            predicted[0] > 0.64 && predicted[0] < 0.66,
            "predicted[0] = {}, expected ~0.65",
            predicted[0]
        );
    }

    #[test]
    fn test_predict_single() {
        let mut service = TemporalKVectorService::new();

        let did = "did:mycelix:agent1".to_string();
        let base_time = Utc::now() - Duration::days(3);

        // Two observations with constant velocity (no acceleration):
        let initial = make_kvec_at([0.5; 8], base_time);
        service.register_agent(&did, initial);

        let update1 = make_kvec_at(
            [0.55, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            base_time + Duration::days(1),
        );
        service.update_observation(&did, update1).unwrap();

        let update2 = make_kvec_at(
            [0.6, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            base_time + Duration::days(2),
        );
        service.update_observation(&did, update2).unwrap();

        // velocity = 0.05/day, accel = 0
        // predict(2 days) = 0.6 + 0.05*2 = 0.7
        let predicted = service.predict(&did, 2.0).unwrap();
        assert!(
            predicted[0] > 0.69 && predicted[0] < 0.71,
            "predicted[0] = {}, expected ~0.7",
            predicted[0]
        );
    }

    #[test]
    fn test_predict_clamped_to_unit_interval() {
        let mut service = TemporalKVectorService::new();

        let did = "did:mycelix:agent1".to_string();
        let initial = make_kvec([0.9; 8], 1);
        service.register_agent(&did, initial);

        let update = make_kvec([1.0, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9], 0);
        service.update_observation(&did, update).unwrap();

        // Predicting far forward should clamp to 1.0
        let predicted = service.predict(&did, 100.0).unwrap();
        assert!(predicted[0] <= 1.0);
        assert!(predicted[0] >= 0.0);
    }

    #[test]
    fn test_most_volatile_dimensions_global() {
        let mut service = TemporalKVectorService::new();

        let did1 = "did:mycelix:agent1".to_string();
        let initial1 = make_kvec([0.5; 8], 1);
        service.register_agent(&did1, initial1);
        let update1 = make_kvec([0.9, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5], 0);
        service.update_observation(&did1, update1).unwrap();

        let did2 = "did:mycelix:agent2".to_string();
        let initial2 = make_kvec([0.5; 8], 1);
        service.register_agent(&did2, initial2);
        let update2 = make_kvec([0.8, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5], 0);
        service.update_observation(&did2, update2).unwrap();

        let volatile = service.most_volatile_dimensions_global(3);
        assert!(!volatile.is_empty());
        // Dim 0 should be the most volatile
        assert_eq!(volatile[0].0, 0);
    }

    #[test]
    fn test_drain_events() {
        let mut service = TemporalKVectorService::new();
        let did = "did:mycelix:agent1".to_string();
        let initial = make_kvec([0.5; 8], 1);
        service.register_agent(&did, initial);

        let update = make_kvec([0.6; 8], 0);
        service.update_observation(&did, update).unwrap();

        let events = service.drain_events();
        assert_eq!(events.len(), 1);

        // After drain, no more events
        let events = service.drain_events();
        assert!(events.is_empty());
    }

    #[test]
    fn test_unregister_agent() {
        let mut service = TemporalKVectorService::new();
        let did = "did:mycelix:agent1".to_string();
        let initial = make_kvec([0.5; 8], 0);
        service.register_agent(&did, initial);

        assert!(service.is_registered(&did));
        let removed = service.unregister_agent(&did);
        assert!(removed.is_some());
        assert!(!service.is_registered(&did));
    }

    #[test]
    fn test_gate1_check_passes_for_valid_data() {
        let mut service = TemporalKVectorService::new();
        let did = "did:mycelix:agent1".to_string();
        let initial = make_kvec([0.5; 8], 1);
        service.register_agent(&did, initial);

        let update = make_kvec([0.6; 8], 0);
        service.update_observation(&did, update).unwrap();

        let checks = service.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_living_primitive_trait() {
        let service = TemporalKVectorService::new();
        assert_eq!(service.primitive_id(), "temporal_k_vector");
        assert_eq!(service.primitive_number(), 5);
        assert_eq!(service.module(), PrimitiveModule::Consciousness);
        assert_eq!(service.tier(), 1);
        assert!(service.is_active_in_phase(CyclePhase::Shadow));
        assert!(service.is_active_in_phase(CyclePhase::EmergentPersonhood));
    }

    #[test]
    fn test_collect_metrics() {
        let mut service = TemporalKVectorService::new();
        let did = "did:mycelix:agent1".to_string();
        let initial = make_kvec([0.5; 8], 0);
        service.register_agent(&did, initial);

        let metrics = service.collect_metrics();
        assert_eq!(metrics["tracked_agents"], 1);
        assert_eq!(metrics["primitive"], "temporal_k_vector");
    }

    #[test]
    fn test_empty_service_volatile_dimensions() {
        let service = TemporalKVectorService::new();
        let volatile = service.most_volatile_dimensions_global(3);
        assert!(volatile.is_empty());
    }

    #[test]
    fn test_multiple_agents_predictions() {
        let mut service = TemporalKVectorService::new();

        for i in 0..5 {
            let did = format!("did:mycelix:agent{}", i);
            let val = 0.1 * (i + 1) as f64;
            let initial = make_kvec([val; 8], 1);
            service.register_agent(&did, initial);

            let new_val = val + 0.05;
            let update = make_kvec([new_val; 8], 0);
            service.update_observation(&did, update).unwrap();
        }

        let predictions = service.predict_all(1.0);
        assert_eq!(predictions.len(), 5);

        // Each prediction should be forward-extrapolated
        for (_, pred) in &predictions {
            for v in pred {
                assert!(*v >= 0.0 && *v <= 1.0);
            }
        }
    }
}
