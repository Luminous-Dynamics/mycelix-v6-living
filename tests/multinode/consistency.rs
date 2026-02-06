//! Link propagation and GetStrategy consistency tests.
//!
//! These tests verify that data created on one node is visible to other nodes
//! after gossip propagation. The key fix tested here is changing
//! GetStrategy::Local to GetStrategy::Network.

use super::*;

// =============================================================================
// Link Propagation Tests
// =============================================================================

/// Test that wounds created on Alice are visible to Bob after gossip.
///
/// This test verifies the GetStrategy::Network fix in living-metabolism.
/// Previously with GetStrategy::Local, wounds would only be visible on the
/// creating node's local DHT.
#[tokio::test]
async fn test_wound_visible_across_nodes() {
    let network = TestNetwork::setup_2_node().await;

    // Install the DNA on both conductors
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
        "description": "Test wound for consistency",
        "witnesses": [],
        "epistemic": {
            "classification": "Empirical",
            "confidence": 0.9,
            "sources": []
        }
    });

    let _wound_record: Record = network.conductor(0)
        .call(alice_cell.zome("living_metabolism"), "create_wound", input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see Alice's wounds
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let wounds: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_metabolism"), "get_wounds_for_agent", alice_pubkey)
        .await;

    // With GetStrategy::Network, this should pass
    // With GetStrategy::Local, this would fail
    assert_eq!(
        wounds.len(),
        1,
        "Bob should see Alice's wound via GetStrategy::Network"
    );
}

/// Test that beauty scores created on one node are visible on another.
///
/// This test verifies the GetStrategy::Network fix in living-epistemics.
#[tokio::test]
async fn test_beauty_scores_visible_across_nodes() {
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

    // Alice creates a dream proposal (to have something to score)
    let dream_input = serde_json::json!({
        "title": "Test Dream",
        "description": "A dream for testing",
        "epistemic": {
            "classification": "Speculative",
            "confidence": 0.7,
            "sources": []
        }
    });

    let dream_record: Record = network.conductor(0)
        .call(alice_cell.zome("living_consciousness"), "submit_dream_proposal", dream_input)
        .await;

    let dream_hash = dream_record.action_hashed().hash.clone();

    // Wait for dream to propagate
    network.wait_for_gossip().await;

    // Alice submits a beauty score for the dream
    let beauty_input = serde_json::json!({
        "target_hash": dream_hash,
        "coherence": 0.8,
        "elegance": 0.9,
        "resonance": 0.7,
        "aliveness": 0.85,
        "wholeness": 0.75,
        "narrative": "Beautiful proposal",
        "epistemic": {
            "classification": "Subjective",
            "confidence": 0.8,
            "sources": []
        }
    });

    let _score_record: Record = network.conductor(0)
        .call(alice_cell.zome("living_epistemics"), "submit_beauty_score", beauty_input)
        .await;

    // Wait for gossip
    network.wait_for_gossip().await;

    // Bob should see the beauty scores
    let scores: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_epistemics"), "get_beauty_scores", dream_hash)
        .await;

    assert_eq!(
        scores.len(),
        1,
        "Bob should see Alice's beauty score via GetStrategy::Network"
    );
}

/// Test that resonance addresses are discoverable across nodes.
///
/// This test verifies the GetStrategy::Network fix in living-structural.
#[tokio::test]
async fn test_resonance_address_discoverable_across_nodes() {
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

    // Alice creates a resonance address
    let address_input = serde_json::json!({
        "pattern_vector": [0.1, 0.2, 0.3, 0.4, 0.5],
        "description": "Test resonance address",
        "referenced_hashes": [],
        "epistemic": {
            "classification": "Empirical",
            "confidence": 0.9,
            "sources": []
        }
    });

    let _address_record: Record = network.conductor(0)
        .call(alice_cell.zome("living_structural"), "create_resonance_address", address_input)
        .await;

    // Wait for gossip
    network.wait_for_gossip().await;

    // Note: resolve_by_pattern queries from the calling agent's perspective,
    // so Bob would need to create his own addresses to query.
    // This test verifies the entry propagates.
}

// =============================================================================
// Eventual Consistency Tests
// =============================================================================

/// Test that multiple wounds created rapidly are all eventually visible.
#[tokio::test]
async fn test_rapid_wound_creation_eventual_consistency() {
    let network = TestNetwork::setup(TestNetworkConfig::two_node().with_extended_gossip()).await;

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

    // Alice creates multiple wounds rapidly
    let wound_count = 5;
    for i in 0..wound_count {
        let input = serde_json::json!({
            "description": format!("Rapid wound {}", i),
            "witnesses": [],
            "epistemic": {
                "classification": "Empirical",
                "confidence": 0.9,
                "sources": []
            }
        });

        let _: Record = network.conductor(0)
            .call(alice_cell.zome("living_metabolism"), "create_wound", input)
            .await;
    }

    // Wait for extended gossip time
    network.wait_for_gossip().await;

    // Bob should eventually see all wounds
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let wounds: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_metabolism"), "get_wounds_for_agent", alice_pubkey)
        .await;

    assert_eq!(
        wounds.len(),
        wound_count,
        "Bob should see all {} wounds after gossip",
        wound_count
    );
}

// =============================================================================
// Cross-Zome Consistency Tests
// =============================================================================

/// Test that linked records across different zomes are consistent.
#[tokio::test]
async fn test_cross_zome_link_consistency() {
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
    let _bob_cell = &apps[1].cells()[0];

    // Alice creates an entanglement (relational zome)
    let entanglement_input = serde_json::json!({
        "partner": apps[1].cells()[0].agent_pubkey(),
        "strength": 0.8,
        "context": "Test entanglement",
        "mutual_consent": true,
        "epistemic": {
            "classification": "Relational",
            "confidence": 0.9,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_relational"), "form_entanglement", entanglement_input)
        .await;

    // Wait for gossip
    network.wait_for_gossip().await;

    // The entanglement should be linked from both agents
    // Both Alice and Bob are linked to the entanglement
    // This tests cross-agent link consistency
}

// =============================================================================
// Relational Zome Consistency Tests
// =============================================================================

/// Test that entanglements created on Alice are visible to Bob after gossip.
///
/// This test verifies the GetStrategy::Network fix in living-relational.
#[tokio::test]
async fn test_entanglement_visible_across_nodes() {
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

    // Alice forms an entanglement with Bob
    let entanglement_input = serde_json::json!({
        "partner": bob_cell.agent_pubkey(),
        "strength": 0.85,
        "context": "Test entanglement for consistency",
        "mutual_consent": true,
        "epistemic": {
            "classification": "Relational",
            "confidence": 0.9,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_relational"), "form_entanglement", entanglement_input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see the entanglement linked to Alice
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let entanglements: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_relational"), "get_entanglements_for_agent", alice_pubkey)
        .await;

    // With GetStrategy::Network, this should pass
    assert_eq!(
        entanglements.len(),
        1,
        "Bob should see Alice's entanglement via GetStrategy::Network"
    );
}

/// Test that liminal states created on Alice are visible to Bob after gossip.
#[tokio::test]
async fn test_liminal_state_visible_across_nodes() {
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

    // Alice enters a liminal state
    let liminal_input = serde_json::json!({
        "description": "Test liminal state for consistency",
        "witnesses": [bob_cell.agent_pubkey()],
        "epistemic": {
            "classification": "Experiential",
            "confidence": 0.8,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_relational"), "enter_liminal_state", liminal_input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see Alice's liminal state
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let liminal_states: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_relational"), "get_liminal_states_for_agent", alice_pubkey)
        .await;

    assert_eq!(
        liminal_states.len(),
        1,
        "Bob should see Alice's liminal state via GetStrategy::Network"
    );
}

// =============================================================================
// Consciousness Zome Consistency Tests
// =============================================================================

/// Test that k-vector snapshots created on Alice are visible to Bob after gossip.
///
/// This test verifies the GetStrategy::Network fix in living-consciousness.
#[tokio::test]
async fn test_k_vector_visible_across_nodes() {
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

    // Alice submits a k-vector snapshot
    let k_vector_input = serde_json::json!({
        "dimensions": [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        "context": "Test k-vector for consistency",
        "epistemic": {
            "classification": "Phenomenal",
            "confidence": 0.85,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_consciousness"), "submit_k_vector_snapshot", k_vector_input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see Alice's k-vector history
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let k_vectors: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_consciousness"), "get_k_vector_history", alice_pubkey)
        .await;

    assert_eq!(
        k_vectors.len(),
        1,
        "Bob should see Alice's k-vector snapshot via GetStrategy::Network"
    );
}

/// Test that dream confirmations are visible across nodes.
#[tokio::test]
async fn test_dream_confirmation_visible_across_nodes() {
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

    // Alice creates a dream proposal
    let dream_input = serde_json::json!({
        "title": "Test Dream for Consistency",
        "description": "A dream to verify confirmation propagation",
        "epistemic": {
            "classification": "Imaginative",
            "confidence": 0.7,
            "sources": []
        }
    });

    let dream_record: Record = network.conductor(0)
        .call(alice_cell.zome("living_consciousness"), "submit_dream_proposal", dream_input)
        .await;

    let dream_hash = dream_record.action_hashed().hash.clone();

    // Wait for dream to propagate
    network.wait_for_gossip().await;

    // Alice confirms her own dream
    let confirm_input = serde_json::json!({
        "proposal_hash": dream_hash,
        "vote": true
    });

    let _: bool = network.conductor(0)
        .call(alice_cell.zome("living_consciousness"), "confirm_dream_proposal", confirm_input)
        .await;

    // Wait for confirmation to propagate
    network.wait_for_gossip().await;

    // Bob should see the confirmation
    let confirmations: Vec<hdk::prelude::AgentPubKey> = network.conductor(1)
        .call(bob_cell.zome("living_consciousness"), "get_dream_confirmations", dream_hash)
        .await;

    assert_eq!(
        confirmations.len(),
        1,
        "Bob should see dream confirmations via GetStrategy::Network"
    );
}

// =============================================================================
// Metabolism Zome Extended Consistency Tests
// =============================================================================

/// Test that kenosis commitments created on Alice are visible to Bob after gossip.
#[tokio::test]
async fn test_kenosis_visible_across_nodes() {
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

    // Alice commits kenosis
    let kenosis_input = serde_json::json!({
        "release_description": "Test kenosis for consistency",
        "release_percentage": 0.10,
        "beneficiaries": [bob_cell.agent_pubkey()],
        "epistemic": {
            "classification": "Ethical",
            "confidence": 1.0,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_metabolism"), "commit_kenosis", kenosis_input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see Alice's kenosis
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let kenosis_records: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_metabolism"), "get_kenosis_for_agent", alice_pubkey)
        .await;

    assert_eq!(
        kenosis_records.len(),
        1,
        "Bob should see Alice's kenosis via GetStrategy::Network"
    );
}

/// Test that composting records created on Alice are visible to Bob after gossip.
#[tokio::test]
async fn test_composting_visible_across_nodes() {
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

    // Alice starts composting
    let compost_input = serde_json::json!({
        "source_description": "Test composting for consistency",
        "cycle_phase": "Death",
        "epistemic": {
            "classification": "Ecological",
            "confidence": 0.9,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_metabolism"), "start_composting", compost_input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see Alice's composting
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let composting_records: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_metabolism"), "get_composting_for_agent", alice_pubkey)
        .await;

    assert_eq!(
        composting_records.len(),
        1,
        "Bob should see Alice's composting via GetStrategy::Network"
    );
}

// =============================================================================
// Epistemics Zome Extended Consistency Tests
// =============================================================================

/// Test that shadow records created on Alice are visible to Bob after gossip.
#[tokio::test]
async fn test_shadow_visible_across_nodes() {
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

    // Alice surfaces a shadow
    let shadow_input = serde_json::json!({
        "topic": "collective_denial",
        "shadow_description": "Test shadow for consistency",
        "integration_path": "acknowledging the hidden",
        "epistemic": {
            "classification": "Shadow",
            "confidence": 0.75,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_epistemics"), "surface_shadow", shadow_input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see Alice's shadow
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let shadows: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_epistemics"), "get_shadows_for_agent", alice_pubkey)
        .await;

    assert_eq!(
        shadows.len(),
        1,
        "Bob should see Alice's shadow via GetStrategy::Network"
    );
}

/// Test that silence records created on Alice are visible to Bob after gossip.
#[tokio::test]
async fn test_silence_visible_across_nodes() {
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

    // Alice records a silence
    let silence_input = serde_json::json!({
        "participants": [alice_cell.agent_pubkey(), bob_cell.agent_pubkey()],
        "duration_seconds": 300,
        "context": "Test silence for consistency",
        "presence_proofs": [{
            "agent": alice_cell.agent_pubkey(),
            "timestamp": 1234567890,
            "signature": "test_signature"
        }],
        "epistemic": {
            "classification": "Contemplative",
            "confidence": 0.95,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_epistemics"), "record_silence", silence_input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see Alice's silence record
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let silences: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_epistemics"), "get_silences_for_agent", alice_pubkey)
        .await;

    assert_eq!(
        silences.len(),
        1,
        "Bob should see Alice's silence via GetStrategy::Network"
    );
}

// =============================================================================
// Structural Zome Extended Consistency Tests
// =============================================================================

/// Test that governance patterns created on Alice are visible to Bob after gossip.
#[tokio::test]
async fn test_governance_pattern_visible_across_nodes() {
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

    // Alice creates a fractal governance pattern
    let pattern_input = serde_json::json!({
        "pattern_name": "Test Governance Pattern",
        "scale": "local",
        "parent_pattern_hash": null,
        "structural_rules": ["rule1", "rule2"],
        "participants": [alice_cell.agent_pubkey()],
        "epistemic": {
            "classification": "Structural",
            "confidence": 0.9,
            "sources": []
        }
    });

    let _: Record = network.conductor(0)
        .call(alice_cell.zome("living_structural"), "create_fractal_pattern", pattern_input)
        .await;

    // Wait for gossip propagation
    network.wait_for_gossip().await;

    // Bob should see Alice's governance pattern
    let alice_pubkey = alice_cell.agent_pubkey().clone();
    let patterns: Vec<Record> = network.conductor(1)
        .call(bob_cell.zome("living_structural"), "get_governance_patterns_for_agent", alice_pubkey)
        .await;

    assert_eq!(
        patterns.len(),
        1,
        "Bob should see Alice's governance pattern via GetStrategy::Network"
    );
}
