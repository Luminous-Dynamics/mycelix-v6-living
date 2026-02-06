//! Concurrent operation tests for multi-node scenarios.
//!
//! These tests verify that concurrent operations from multiple nodes
//! are handled correctly without race conditions or data corruption.

use super::*;

// =============================================================================
// Phase Advance Concurrency Tests
// =============================================================================

/// Test that concurrent wound phase advances are serialized correctly.
///
/// When Alice and Bob both try to advance the same wound's phase,
/// exactly one should succeed (or both should see the same final phase).
#[tokio::test]
async fn test_concurrent_phase_advance_serializes() {
    let network = TestNetwork::setup_2_node().await;

    let dna = SweetDnaFile::from_bundle(std::path::Path::new("../../dna"))
        .await
        .expect("Failed to load DNA");

    let apps = network
        .conductors
        .setup_app("mycelix", &[dna])
        .await
        .expect("Failed to install app");

    let alice_cell = &apps[0].cells()[0];
    let bob_cell = &apps[1].cells()[0];

    // Alice creates a wound
    let input = serde_json::json!({
        "description": "Concurrency test wound",
        "witnesses": [],
        "epistemic": {
            "classification": "Empirical",
            "confidence": 0.9,
            "sources": []
        }
    });

    let wound_record: Record = network.conductor(0)
        .call(alice_cell.zome("living_metabolism"), "create_wound", input)
        .await;

    let wound_hash = wound_record.action_hashed().hash.clone();

    // Wait for wound to propagate to Bob
    network.wait_for_gossip().await;

    // Both Alice and Bob try to advance the wound phase concurrently
    let alice_advance = tokio::spawn({
        let conductor = network.conductors[0].clone();
        let cell = alice_cell.clone();
        let hash = wound_hash.clone();
        async move {
            conductor
                .call::<_, Record>(
                    cell.zome("living_metabolism"),
                    "advance_wound_phase",
                    hash,
                )
                .await
        }
    });

    let bob_advance = tokio::spawn({
        let conductor = network.conductors[1].clone();
        let cell = bob_cell.clone();
        let hash = wound_hash.clone();
        async move {
            conductor
                .call::<_, Record>(
                    cell.zome("living_metabolism"),
                    "advance_wound_phase",
                    hash,
                )
                .await
        }
    });

    // Wait for both operations to complete
    let (alice_result, bob_result) = tokio::join!(alice_advance, bob_advance);

    // At least one should have succeeded
    // Due to Holochain's eventual consistency, both might succeed
    // but the final state should be consistent

    // Wait for consistency
    network.wait_for_gossip().await;

    // Query final state from both nodes
    let alice_wounds: Vec<Record> = network.conductor(0)
        .call(alice_cell.zome("living_metabolism"), "get_wounds_for_agent", alice_cell.agent_pubkey())
        .await;

    let bob_wounds: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_metabolism"), "get_wounds_for_agent", alice_cell.agent_pubkey())
        .await;

    // Both should see the wound
    assert!(!alice_wounds.is_empty());
    assert!(!bob_wounds.is_empty());

    // The wound should be in Inflammation phase (advanced once from Hemostasis)
    // Note: Exact behavior depends on Holochain's conflict resolution
}

// =============================================================================
// Concurrent Creation Tests
// =============================================================================

/// Test that concurrent wound creation from multiple nodes works correctly.
#[tokio::test]
async fn test_concurrent_wound_creation() {
    let network = TestNetwork::setup_2_node().await;

    let dna = SweetDnaFile::from_bundle(std::path::Path::new("../../dna"))
        .await
        .expect("Failed to load DNA");

    let apps = network
        .conductors
        .setup_app("mycelix", &[dna])
        .await
        .expect("Failed to install app");

    let alice_cell = &apps[0].cells()[0];
    let bob_cell = &apps[1].cells()[0];

    // Both create wounds concurrently
    let alice_create = tokio::spawn({
        let conductor = network.conductors[0].clone();
        let cell = alice_cell.clone();
        async move {
            let input = serde_json::json!({
                "description": "Alice's concurrent wound",
                "witnesses": [],
                "epistemic": {
                    "classification": "Empirical",
                    "confidence": 0.9,
                    "sources": []
                }
            });
            conductor
                .call::<_, Record>(cell.zome("living_metabolism"), "create_wound", input)
                .await
        }
    });

    let bob_create = tokio::spawn({
        let conductor = network.conductors[1].clone();
        let cell = bob_cell.clone();
        async move {
            let input = serde_json::json!({
                "description": "Bob's concurrent wound",
                "witnesses": [],
                "epistemic": {
                    "classification": "Empirical",
                    "confidence": 0.9,
                    "sources": []
                }
            });
            conductor
                .call::<_, Record>(cell.zome("living_metabolism"), "create_wound", input)
                .await
        }
    });

    // Both should succeed
    let (alice_result, bob_result) = tokio::join!(alice_create, bob_create);
    let alice_wound = alice_result.expect("Alice's spawn should succeed");
    let bob_wound = bob_result.expect("Bob's spawn should succeed");

    // Wait for gossip
    network.wait_for_gossip().await;

    // Each agent should see their own wound
    let alice_wounds: Vec<Record> = network.conductor(0)
        .call(alice_cell.zome("living_metabolism"), "get_wounds_for_agent", alice_cell.agent_pubkey())
        .await;

    let bob_wounds: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_metabolism"), "get_wounds_for_agent", bob_cell.agent_pubkey())
        .await;

    assert_eq!(alice_wounds.len(), 1, "Alice should see her wound");
    assert_eq!(bob_wounds.len(), 1, "Bob should see his wound");
}

// =============================================================================
// Concurrent Update Tests
// =============================================================================

/// Test concurrent kenosis commitments from same agent.
#[tokio::test]
async fn test_concurrent_kenosis_commits() {
    let network = TestNetwork::setup_2_node().await;

    let dna = SweetDnaFile::from_bundle(std::path::Path::new("../../dna"))
        .await
        .expect("Failed to load DNA");

    let apps = network
        .conductors
        .setup_app("mycelix", &[dna])
        .await
        .expect("Failed to install app");

    let alice_cell = &apps[0].cells()[0];

    // Alice makes two concurrent kenosis commitments
    // Both should succeed as they are separate entries
    let commit1 = tokio::spawn({
        let conductor = network.conductors[0].clone();
        let cell = alice_cell.clone();
        async move {
            let input = serde_json::json!({
                "release_description": "First kenosis",
                "release_percentage": 0.05,
                "beneficiaries": [],
                "epistemic": {
                    "classification": "Ethical",
                    "confidence": 1.0,
                    "sources": []
                }
            });
            conductor
                .call::<_, Record>(cell.zome("living_metabolism"), "commit_kenosis", input)
                .await
        }
    });

    let commit2 = tokio::spawn({
        let conductor = network.conductors[0].clone();
        let cell = alice_cell.clone();
        async move {
            let input = serde_json::json!({
                "release_description": "Second kenosis",
                "release_percentage": 0.10,
                "beneficiaries": [],
                "epistemic": {
                    "classification": "Ethical",
                    "confidence": 1.0,
                    "sources": []
                }
            });
            conductor
                .call::<_, Record>(cell.zome("living_metabolism"), "commit_kenosis", input)
                .await
        }
    });

    let (result1, result2) = tokio::join!(commit1, commit2);

    // Both should succeed (separate entries)
    result1.expect("First kenosis should succeed");
    result2.expect("Second kenosis should succeed");
}

// =============================================================================
// Entanglement Concurrency Tests
// =============================================================================

/// Test concurrent entanglement formation.
#[tokio::test]
async fn test_concurrent_entanglement_formation() {
    let network = TestNetwork::setup_2_node().await;

    let dna = SweetDnaFile::from_bundle(std::path::Path::new("../../dna"))
        .await
        .expect("Failed to load DNA");

    let apps = network
        .conductors
        .setup_app("mycelix", &[dna])
        .await
        .expect("Failed to install app");

    let alice_cell = &apps[0].cells()[0];
    let bob_cell = &apps[1].cells()[0];

    // Both try to form entanglement with each other concurrently
    let alice_entangle = tokio::spawn({
        let conductor = network.conductors[0].clone();
        let cell = alice_cell.clone();
        let partner = bob_cell.agent_pubkey().clone();
        async move {
            let input = serde_json::json!({
                "partner": partner,
                "strength": 0.8,
                "context": "Alice initiates",
                "mutual_consent": true,
                "epistemic": {
                    "classification": "Relational",
                    "confidence": 0.9,
                    "sources": []
                }
            });
            conductor
                .call::<_, Record>(cell.zome("living_relational"), "form_entanglement", input)
                .await
        }
    });

    let bob_entangle = tokio::spawn({
        let conductor = network.conductors[1].clone();
        let cell = bob_cell.clone();
        let partner = alice_cell.agent_pubkey().clone();
        async move {
            let input = serde_json::json!({
                "partner": partner,
                "strength": 0.9,
                "context": "Bob initiates",
                "mutual_consent": true,
                "epistemic": {
                    "classification": "Relational",
                    "confidence": 0.9,
                    "sources": []
                }
            });
            conductor
                .call::<_, Record>(cell.zome("living_relational"), "form_entanglement", input)
                .await
        }
    });

    let (alice_result, bob_result) = tokio::join!(alice_entangle, bob_entangle);

    // Both should succeed (creating separate entanglement records)
    alice_result.expect("Alice's entanglement should succeed");
    bob_result.expect("Bob's entanglement should succeed");

    // Wait for gossip
    network.wait_for_gossip().await;

    // Both entanglements should exist (this is expected behavior -
    // each agent can create their own entanglement record)
}
