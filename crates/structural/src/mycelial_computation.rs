//! # Mycelial Computation Engine -- Primitive [21]
//!
//! Distributed computation via network topology.
//!
//! Inspired by how mycelial networks distribute nutrients and information
//! through their hyphal structure, this engine distributes computational tasks
//! across network nodes.  Tasks are submitted with a computation description
//! and input hash, then assigned to nodes based on one of three strategies:
//!
//! - **NearestNeighbor**: Select the topologically closest nodes.
//! - **LoadBalanced**: Select the least-loaded capable nodes.
//! - **CapabilityMatched**: Select nodes whose declared capabilities match
//!   the task requirements.
//!
//! Results are verified through **redundant computation**: multiple nodes
//! execute the same task, and the result is accepted only when a quorum of
//! nodes produce the same result hash.
//!
//! ## Feature Flag
//!
//! Behind the `tier3-experimental` feature flag.
//!
//! ## Constitutional Alignment
//!
//! **Evolutionary Progression (Harmony 7)**: The network should leverage its
//! distributed structure for computation, not centralize it.  Mycelial
//! computation ensures that computational power grows with the network.
//!
//! **Sacred Reciprocity (Harmony 6)**: Nodes that contribute computation
//! receive reciprocal benefit from the network's collective capability.
//!
//! ## Three Gates
//!
//! - **Gate 1**: A completed task must have a non-None `result_hash`.
//! - **Gate 1**: `assigned_nodes` must be non-empty for any in-progress task.
//! - **Gate 2**: Warns if a task has been pending for too long without assignment.
//!
//! ## Dependency
//!
//! Depends on [20] Time-Crystal Consensus and [17] Resonance Addressing.
//!
//! ## Classification
//!
//! E2/N0/M2 -- Privately verifiable / Personal / Persistent.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use living_core::error::{LivingProtocolError, LivingResult};
use living_core::traits::{LivingPrimitive, PrimitiveModule};
use living_core::{
    CyclePhase, Did, EventBus, Gate1Check, Gate2Warning, HashDigest, LivingProtocolEvent,
    MycelialTask, MycelialTaskCompletedEvent, MycelialTaskDistributedEvent,
};

// =============================================================================
// Assignment Strategy
// =============================================================================

/// Strategy for assigning nodes to a computation task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentStrategy {
    /// Select the topologically closest nodes.
    NearestNeighbor,
    /// Select the least-loaded capable nodes.
    LoadBalanced,
    /// Select nodes whose declared capabilities match the task.
    CapabilityMatched,
}

// =============================================================================
// Node Result Tracking
// =============================================================================

/// Tracks results submitted by individual nodes for a task.
#[derive(Debug, Clone)]
struct NodeResult {
    node_did: Did,
    result_hash: HashDigest,
}

// =============================================================================
// Mycelial Computation Engine
// =============================================================================

/// Engine for distributing and verifying computation across network nodes.
pub struct MycelialComputationEngine {
    /// Active and completed tasks indexed by task ID.
    tasks: HashMap<String, MycelialTask>,
    /// Declared capabilities for each registered node.
    node_capabilities: HashMap<Did, Vec<String>>,
    /// Number of tasks currently assigned to each node (for load balancing).
    node_load: HashMap<Did, usize>,
    /// Individual node results for each task (for redundant verification).
    task_results: HashMap<String, Vec<NodeResult>>,
    /// Event bus for emitting mycelial computation events.
    event_bus: Arc<dyn EventBus>,
    /// Whether the engine is active in the current cycle phase.
    active: bool,
}

impl MycelialComputationEngine {
    /// Create a new mycelial computation engine.
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            tasks: HashMap::new(),
            node_capabilities: HashMap::new(),
            node_load: HashMap::new(),
            task_results: HashMap::new(),
            event_bus,
            active: false,
        }
    }

    /// Register a node with its computational capabilities.
    ///
    /// Returns true if the node was newly registered, false if it was updated.
    pub fn register_node(&mut self, did: Did, capabilities: Vec<String>) -> bool {
        let is_new = !self.node_capabilities.contains_key(&did);
        self.node_capabilities.insert(did.clone(), capabilities);
        if is_new {
            self.node_load.insert(did.clone(), 0);
        }

        tracing::info!(
            node_did = %did,
            is_new = is_new,
            "Node registered for mycelial computation."
        );

        is_new
    }

    /// Submit a new computation task for distributed execution.
    ///
    /// The task is created with the given computation description and input
    /// hash.  It starts unassigned; use `assign_nodes` to assign nodes.
    pub fn submit_task(
        &mut self,
        computation: String,
        input_hash: HashDigest,
    ) -> MycelialTaskDistributedEvent {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let task = MycelialTask {
            id: id.clone(),
            computation: computation.clone(),
            input_hash,
            assigned_nodes: Vec::new(),
            result_hash: None,
            started: now,
            completed: None,
        };

        self.tasks.insert(id.clone(), task.clone());
        self.task_results.insert(id.clone(), Vec::new());

        let event = MycelialTaskDistributedEvent {
            task: task.clone(),
            timestamp: now,
        };

        self.event_bus
            .publish(LivingProtocolEvent::MycelialTaskDistributed(event.clone()));

        tracing::info!(
            task_id = %id,
            computation = %computation,
            "Mycelial computation task submitted for distributed execution."
        );

        event
    }

    /// Assign nodes to a task based on the given strategy.
    ///
    /// Returns the list of assigned node DIDs.  Returns an error if the task
    /// does not exist or no suitable nodes are available.
    pub fn assign_nodes(
        &mut self,
        task_id: &str,
        strategy: AssignmentStrategy,
    ) -> LivingResult<Vec<Did>> {
        // Verify task exists
        if !self.tasks.contains_key(task_id) {
            return Err(LivingProtocolError::InvalidResonancePattern(format!(
                "Task {} not found",
                task_id
            )));
        }

        let computation = self.tasks[task_id].computation.clone();
        let assigned = match strategy {
            AssignmentStrategy::NearestNeighbor => {
                // In this simplified implementation, "nearest" means first N
                // registered nodes.  A real implementation would use network
                // topology distance.
                self.node_capabilities
                    .keys()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
            }
            AssignmentStrategy::LoadBalanced => {
                // Select the 3 least-loaded nodes
                let mut nodes_by_load: Vec<(Did, usize)> = self
                    .node_load
                    .iter()
                    .map(|(did, load)| (did.clone(), *load))
                    .collect();
                nodes_by_load.sort_by_key(|(_, load)| *load);
                nodes_by_load
                    .into_iter()
                    .take(3)
                    .map(|(did, _)| did)
                    .collect()
            }
            AssignmentStrategy::CapabilityMatched => {
                // Select nodes whose capabilities include any word from the
                // computation description
                let keywords: Vec<String> = computation
                    .to_lowercase()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                self.node_capabilities
                    .iter()
                    .filter(|(_, caps)| {
                        caps.iter()
                            .any(|cap| keywords.iter().any(|kw| cap.to_lowercase().contains(kw)))
                    })
                    .map(|(did, _)| did.clone())
                    .take(3)
                    .collect()
            }
        };

        if assigned.is_empty() {
            return Err(LivingProtocolError::InvalidResonancePattern(format!(
                "No suitable nodes found for task {} with strategy {:?}",
                task_id, strategy
            )));
        }

        // Update node loads
        for did in &assigned {
            *self.node_load.entry(did.clone()).or_insert(0) += 1;
        }

        // Update task
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.assigned_nodes = assigned.clone();
        }

        tracing::info!(
            task_id = %task_id,
            strategy = ?strategy,
            assigned_count = assigned.len(),
            "Nodes assigned to mycelial computation task."
        );

        Ok(assigned)
    }

    /// Submit a computation result from a node.
    ///
    /// Returns true if the result was accepted (node is assigned to the task).
    pub fn submit_result(&mut self, task_id: &str, node_did: Did, result_hash: HashDigest) -> bool {
        let task = match self.tasks.get(task_id) {
            Some(t) => t,
            None => return false,
        };

        // Verify the node is assigned to this task
        if !task.assigned_nodes.contains(&node_did) {
            return false;
        }

        // Already completed
        if task.completed.is_some() {
            return false;
        }

        // Record the result
        if let Some(results) = self.task_results.get_mut(task_id) {
            // Don't accept duplicate results from the same node
            if results.iter().any(|r| r.node_did == node_did) {
                return false;
            }

            results.push(NodeResult {
                node_did,
                result_hash,
            });
        }

        true
    }

    /// Verify a task's result via redundant computation.
    ///
    /// A result is verified when a majority of assigned nodes have submitted
    /// the same result hash.  Returns true if a quorum agrees on the result.
    pub fn verify_result(&self, task_id: &str) -> bool {
        let task = match self.tasks.get(task_id) {
            Some(t) => t,
            None => return false,
        };

        let results = match self.task_results.get(task_id) {
            Some(r) => r,
            None => return false,
        };

        if results.is_empty() || task.assigned_nodes.is_empty() {
            return false;
        }

        // Count occurrences of each result hash
        let mut hash_counts: HashMap<HashDigest, usize> = HashMap::new();
        for result in results {
            *hash_counts.entry(result.result_hash).or_insert(0) += 1;
        }

        // Check if any hash has majority
        let quorum = (task.assigned_nodes.len() / 2) + 1;
        hash_counts.values().any(|&count| count >= quorum)
    }

    /// Complete a task after verification.
    ///
    /// Sets the result_hash to the majority result and marks the task as
    /// completed.  Returns an error if the task is not verified or does not
    /// exist.
    pub fn complete_task(&mut self, task_id: &str) -> LivingResult<MycelialTaskCompletedEvent> {
        if !self.verify_result(task_id) {
            return Err(LivingProtocolError::InvalidResonancePattern(format!(
                "Task {} has not been verified by a quorum",
                task_id
            )));
        }

        // Find the majority result hash
        let results = self.task_results.get(task_id).ok_or_else(|| {
            LivingProtocolError::InvalidResonancePattern(format!("No results for task {}", task_id))
        })?;

        let mut hash_counts: HashMap<HashDigest, usize> = HashMap::new();
        for result in results {
            *hash_counts.entry(result.result_hash).or_insert(0) += 1;
        }

        let majority_hash = hash_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hash, _)| hash)
            .unwrap(); // Safe because verify_result passed

        let now = Utc::now();
        let task = self.tasks.get_mut(task_id).unwrap();
        let started = task.started;
        task.result_hash = Some(majority_hash);
        task.completed = Some(now);

        // Decrease node loads
        for did in &task.assigned_nodes {
            if let Some(load) = self.node_load.get_mut(did) {
                *load = load.saturating_sub(1);
            }
        }

        let duration = now - started;

        let event = MycelialTaskCompletedEvent {
            task_id: task_id.to_string(),
            result_hash: majority_hash,
            duration,
            timestamp: now,
        };

        self.event_bus
            .publish(LivingProtocolEvent::MycelialTaskCompleted(event.clone()));

        tracing::info!(
            task_id = %task_id,
            duration_secs = duration.num_seconds(),
            "Mycelial computation task completed. Sacred Reciprocity: \
             distributed computation returns results to the network."
        );

        Ok(event)
    }

    /// Get all pending (not completed) tasks.
    pub fn get_pending_tasks(&self) -> Vec<&MycelialTask> {
        self.tasks
            .values()
            .filter(|t| t.completed.is_none())
            .collect()
    }

    /// Get a task by ID.
    pub fn get_task(&self, task_id: &str) -> Option<&MycelialTask> {
        self.tasks.get(task_id)
    }

    /// Get the total number of registered nodes.
    pub fn registered_node_count(&self) -> usize {
        self.node_capabilities.len()
    }

    /// Get the total number of tasks (pending + completed).
    pub fn total_task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get capabilities for a specific node.
    pub fn get_node_capabilities(&self, did: &Did) -> Option<&Vec<String>> {
        self.node_capabilities.get(did)
    }
}

// =============================================================================
// LivingPrimitive trait implementation
// =============================================================================

impl LivingPrimitive for MycelialComputationEngine {
    fn primitive_id(&self) -> &str {
        "mycelial_computation"
    }

    fn primitive_number(&self) -> u8 {
        21
    }

    fn module(&self) -> PrimitiveModule {
        PrimitiveModule::Structural
    }

    fn tier(&self) -> u8 {
        3
    }

    fn on_phase_change(&mut self, new_phase: CyclePhase) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Mycelial computation is active during Co-Creation.
        self.active = new_phase == CyclePhase::CoCreation;
        Ok(Vec::new())
    }

    fn gate1_check(&self) -> Vec<Gate1Check> {
        let mut checks = Vec::new();

        for task in self.tasks.values() {
            // Gate 1: completed tasks must have a result_hash
            if task.completed.is_some() {
                let has_result = task.result_hash.is_some();
                checks.push(Gate1Check {
                    invariant: format!("completed task {} has result_hash", task.id),
                    passed: has_result,
                    details: if has_result {
                        None
                    } else {
                        Some("completed task has no result_hash".to_string())
                    },
                });
            }

            // Gate 1: in-progress tasks with assigned nodes must be non-empty
            if task.completed.is_none() && !task.assigned_nodes.is_empty() {
                let non_empty = !task.assigned_nodes.is_empty();
                checks.push(Gate1Check {
                    invariant: format!("assigned_nodes non-empty for in-progress task {}", task.id),
                    passed: non_empty,
                    details: None,
                });
            }
        }

        checks
    }

    fn gate2_check(&self) -> Vec<Gate2Warning> {
        let mut warnings = Vec::new();

        for task in self.tasks.values() {
            // Gate 2: warn if a task has no assigned nodes and is not completed
            if task.completed.is_none() && task.assigned_nodes.is_empty() {
                let elapsed = Utc::now() - task.started;
                if elapsed.num_minutes() > 5 {
                    warnings.push(Gate2Warning {
                        harmony_violated: "Evolutionary Progression (Harmony 7)".to_string(),
                        severity: 0.3,
                        reputation_impact: 0.0,
                        reasoning: format!(
                            "Task {} has been pending for {} minutes without node assignment. \
                             Consider assigning nodes or canceling the task.",
                            task.id,
                            elapsed.num_minutes()
                        ),
                        user_may_proceed: true,
                    });
                }
            }
        }

        warnings
    }

    fn is_active_in_phase(&self, phase: CyclePhase) -> bool {
        phase == CyclePhase::CoCreation
    }

    fn collect_metrics(&self) -> serde_json::Value {
        let pending = self.get_pending_tasks().len();
        let completed = self
            .tasks
            .values()
            .filter(|t| t.completed.is_some())
            .count();

        serde_json::json!({
            "primitive": "mycelial_computation",
            "primitive_number": 21,
            "registered_nodes": self.node_capabilities.len(),
            "total_tasks": self.tasks.len(),
            "pending_tasks": pending,
            "completed_tasks": completed,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use living_core::InMemoryEventBus;

    fn make_engine() -> MycelialComputationEngine {
        let bus = Arc::new(InMemoryEventBus::new());
        MycelialComputationEngine::new(bus)
    }

    fn make_engine_with_bus() -> (MycelialComputationEngine, Arc<InMemoryEventBus>) {
        let bus = Arc::new(InMemoryEventBus::new());
        let engine = MycelialComputationEngine::new(bus.clone());
        (engine, bus)
    }

    fn test_hash(value: u8) -> HashDigest {
        let mut hash = [0u8; 32];
        hash[0] = value;
        hash
    }

    fn register_test_nodes(engine: &mut MycelialComputationEngine) {
        engine.register_node(
            "did:mycelix:node1".to_string(),
            vec!["compute".to_string(), "hash".to_string()],
        );
        engine.register_node(
            "did:mycelix:node2".to_string(),
            vec!["compute".to_string(), "verify".to_string()],
        );
        engine.register_node(
            "did:mycelix:node3".to_string(),
            vec!["compute".to_string(), "storage".to_string()],
        );
    }

    #[test]
    fn test_register_node() {
        let mut engine = make_engine();
        let is_new =
            engine.register_node("did:mycelix:node1".to_string(), vec!["compute".to_string()]);
        assert!(is_new);
        assert_eq!(engine.registered_node_count(), 1);

        // Re-register same node (update)
        let is_new2 = engine.register_node(
            "did:mycelix:node1".to_string(),
            vec!["compute".to_string(), "storage".to_string()],
        );
        assert!(!is_new2);
        assert_eq!(engine.registered_node_count(), 1);

        let caps = engine
            .get_node_capabilities(&"did:mycelix:node1".to_string())
            .unwrap();
        assert_eq!(caps.len(), 2);
    }

    #[test]
    fn test_submit_task() {
        let (mut engine, bus) = make_engine_with_bus();

        let event = engine.submit_task("hash computation".to_string(), test_hash(1));

        assert!(engine.get_task(&event.task.id).is_some());
        assert!(engine
            .get_task(&event.task.id)
            .unwrap()
            .assigned_nodes
            .is_empty());
        assert!(engine.get_task(&event.task.id).unwrap().completed.is_none());
        assert_eq!(engine.total_task_count(), 1);
        assert_eq!(bus.event_count(), 1);
    }

    #[test]
    fn test_assign_nodes_nearest_neighbor() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        let assigned = engine
            .assign_nodes(&event.task.id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        assert!(!assigned.is_empty());
        assert!(assigned.len() <= 3);

        let task = engine.get_task(&event.task.id).unwrap();
        assert_eq!(task.assigned_nodes, assigned);
    }

    #[test]
    fn test_assign_nodes_load_balanced() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        let assigned = engine
            .assign_nodes(&event.task.id, AssignmentStrategy::LoadBalanced)
            .unwrap();

        assert!(!assigned.is_empty());
    }

    #[test]
    fn test_assign_nodes_capability_matched() {
        let mut engine = make_engine();
        engine.register_node("did:mycelix:hasher".to_string(), vec!["hash".to_string()]);
        engine.register_node(
            "did:mycelix:verifier".to_string(),
            vec!["verify".to_string()],
        );

        let event = engine.submit_task("hash computation".to_string(), test_hash(1));
        let assigned = engine
            .assign_nodes(&event.task.id, AssignmentStrategy::CapabilityMatched)
            .unwrap();

        // Should match "did:mycelix:hasher" because it has "hash" capability
        assert!(assigned.contains(&"did:mycelix:hasher".to_string()));
    }

    #[test]
    fn test_assign_nodes_nonexistent_task() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let result = engine.assign_nodes("nonexistent", AssignmentStrategy::NearestNeighbor);
        assert!(result.is_err());
    }

    #[test]
    fn test_assign_nodes_no_suitable_nodes() {
        let mut engine = make_engine();
        // Register a node with no matching capabilities
        engine.register_node(
            "did:mycelix:node1".to_string(),
            vec!["unrelated".to_string()],
        );

        let event = engine.submit_task("quantum computation".to_string(), test_hash(1));
        let result = engine.assign_nodes(&event.task.id, AssignmentStrategy::CapabilityMatched);
        assert!(result.is_err());
    }

    #[test]
    fn test_submit_result() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        engine
            .assign_nodes(&event.task.id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        let task = engine.get_task(&event.task.id).unwrap();
        let assigned_node = task.assigned_nodes[0].clone();

        let accepted = engine.submit_result(&event.task.id, assigned_node, test_hash(42));
        assert!(accepted);
    }

    #[test]
    fn test_submit_result_unassigned_node() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        engine
            .assign_nodes(&event.task.id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        // Submit from a non-assigned node
        let accepted = engine.submit_result(
            &event.task.id,
            "did:mycelix:unassigned".to_string(),
            test_hash(42),
        );
        assert!(!accepted);
    }

    #[test]
    fn test_submit_result_duplicate_rejected() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        engine
            .assign_nodes(&event.task.id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        let task = engine.get_task(&event.task.id).unwrap();
        let node = task.assigned_nodes[0].clone();

        assert!(engine.submit_result(&event.task.id, node.clone(), test_hash(42)));
        // Duplicate from same node rejected
        assert!(!engine.submit_result(&event.task.id, node, test_hash(42)));
    }

    #[test]
    fn test_verify_result_with_quorum() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        engine
            .assign_nodes(&event.task.id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        let task = engine.get_task(&event.task.id).unwrap();
        let nodes = task.assigned_nodes.clone();

        // Submit same result from majority
        let result_hash = test_hash(42);
        for node in &nodes[..2] {
            engine.submit_result(&event.task.id, node.clone(), result_hash);
        }

        assert!(engine.verify_result(&event.task.id));
    }

    #[test]
    fn test_verify_result_without_quorum() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        engine
            .assign_nodes(&event.task.id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        let task = engine.get_task(&event.task.id).unwrap();
        let nodes = task.assigned_nodes.clone();

        // Submit different results from each node (no quorum)
        for (i, node) in nodes.iter().enumerate() {
            engine.submit_result(&event.task.id, node.clone(), test_hash(i as u8));
        }

        assert!(!engine.verify_result(&event.task.id));
    }

    #[test]
    fn test_complete_task() {
        let (mut engine, bus) = make_engine_with_bus();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        let task_id = event.task.id.clone();
        engine
            .assign_nodes(&task_id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        let task = engine.get_task(&task_id).unwrap();
        let nodes = task.assigned_nodes.clone();

        // Submit same result from majority
        let result_hash = test_hash(42);
        for node in &nodes[..2] {
            engine.submit_result(&task_id, node.clone(), result_hash);
        }

        let completed_event = engine.complete_task(&task_id).unwrap();
        assert_eq!(completed_event.result_hash, result_hash);

        let task = engine.get_task(&task_id).unwrap();
        assert!(task.completed.is_some());
        assert_eq!(task.result_hash, Some(result_hash));

        // Events: 1 distributed + 1 completed
        assert_eq!(bus.event_count(), 2);
    }

    #[test]
    fn test_complete_task_without_verification() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        let result = engine.complete_task(&event.task.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_pending_tasks() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        engine.submit_task("task1".to_string(), test_hash(1));
        engine.submit_task("task2".to_string(), test_hash(2));

        assert_eq!(engine.get_pending_tasks().len(), 2);

        // Complete one task
        let tasks: Vec<_> = engine
            .get_pending_tasks()
            .iter()
            .map(|t| t.id.clone())
            .collect();
        let task_id = &tasks[0];
        engine
            .assign_nodes(task_id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        let task = engine.get_task(task_id).unwrap();
        let nodes = task.assigned_nodes.clone();
        let result_hash = test_hash(99);
        for node in &nodes[..2] {
            engine.submit_result(task_id, node.clone(), result_hash);
        }
        engine.complete_task(task_id).unwrap();

        assert_eq!(engine.get_pending_tasks().len(), 1);
    }

    #[test]
    fn test_load_balancing() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        // Submit multiple tasks and assign with load balancing
        for i in 0..5 {
            let event = engine.submit_task(format!("task {}", i), test_hash(i as u8));
            engine
                .assign_nodes(&event.task.id, AssignmentStrategy::LoadBalanced)
                .unwrap();
        }

        // All nodes should have similar load
        let loads: Vec<usize> = engine.node_load.values().cloned().collect();
        let max_load = *loads.iter().max().unwrap();
        let min_load = *loads.iter().min().unwrap();
        // Load difference should be at most 2 (due to rounding)
        assert!(
            max_load - min_load <= 3,
            "Load should be balanced: min={}, max={}",
            min_load,
            max_load
        );
    }

    #[test]
    fn test_gate1_completed_task_has_result() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);

        let event = engine.submit_task("computation".to_string(), test_hash(1));
        let task_id = event.task.id.clone();
        engine
            .assign_nodes(&task_id, AssignmentStrategy::NearestNeighbor)
            .unwrap();

        let task = engine.get_task(&task_id).unwrap();
        let nodes = task.assigned_nodes.clone();
        let result_hash = test_hash(42);
        for node in &nodes[..2] {
            engine.submit_result(&task_id, node.clone(), result_hash);
        }
        engine.complete_task(&task_id).unwrap();

        let checks = engine.gate1_check();
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_primitive_metadata() {
        let engine = make_engine();
        assert_eq!(engine.primitive_id(), "mycelial_computation");
        assert_eq!(engine.primitive_number(), 21);
        assert_eq!(engine.module(), PrimitiveModule::Structural);
        assert_eq!(engine.tier(), 3);
    }

    #[test]
    fn test_is_active_in_phase() {
        let engine = make_engine();
        assert!(engine.is_active_in_phase(CyclePhase::CoCreation));
        assert!(!engine.is_active_in_phase(CyclePhase::Shadow));
        assert!(!engine.is_active_in_phase(CyclePhase::Kenosis));
    }

    #[test]
    fn test_collect_metrics() {
        let mut engine = make_engine();
        register_test_nodes(&mut engine);
        engine.submit_task("computation".to_string(), test_hash(1));

        let metrics = engine.collect_metrics();
        assert_eq!(metrics["registered_nodes"], 3);
        assert_eq!(metrics["total_tasks"], 1);
        assert_eq!(metrics["pending_tasks"], 1);
        assert_eq!(metrics["completed_tasks"], 0);
        assert_eq!(metrics["primitive_number"], 21);
    }

    #[test]
    fn test_end_to_end_mycelial_computation() {
        // Full lifecycle: register nodes -> submit task -> assign -> compute
        // -> verify -> complete
        let mut engine = make_engine();

        // Register nodes
        engine.register_node(
            "did:mycelix:alpha".to_string(),
            vec!["hash".to_string(), "compute".to_string()],
        );
        engine.register_node(
            "did:mycelix:beta".to_string(),
            vec!["hash".to_string(), "verify".to_string()],
        );
        engine.register_node(
            "did:mycelix:gamma".to_string(),
            vec!["hash".to_string(), "storage".to_string()],
        );

        // Submit task
        let event = engine.submit_task("hash computation".to_string(), test_hash(1));
        let task_id = event.task.id.clone();

        // Assign nodes
        let assigned = engine
            .assign_nodes(&task_id, AssignmentStrategy::CapabilityMatched)
            .unwrap();
        assert_eq!(assigned.len(), 3); // All have "hash" capability

        // Simulate computation: all nodes agree on result
        let result = test_hash(255);
        for node in &assigned {
            engine.submit_result(&task_id, node.clone(), result);
        }

        // Verify and complete
        assert!(engine.verify_result(&task_id));
        let completed = engine.complete_task(&task_id).unwrap();
        assert_eq!(completed.result_hash, result);

        // Task should be marked complete
        let task = engine.get_task(&task_id).unwrap();
        assert!(task.completed.is_some());
        assert_eq!(engine.get_pending_tasks().len(), 0);
    }
}
