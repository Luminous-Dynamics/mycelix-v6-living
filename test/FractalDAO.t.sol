// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../contracts/FractalDAO.sol";

contract FractalDAOTest is Test {
    FractalDAO public dao;

    address public admin = address(this);
    address public voter1 = address(0xBEEF);
    address public voter2 = address(0xCAFE);
    address public voter3 = address(0xDEAD);

    function setUp() public {
        dao = new FractalDAO();
    }

    // =========================================================================
    // Pattern Creation
    // =========================================================================

    function test_createPattern() public {
        bytes32 patternId = keccak256("pattern-1");
        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Community,
            5000, // 50% quorum
            6667, // 66.67% supermajority
            FractalDAO.DecisionMechanism.Consensus
        );

        FractalDAO.GovernancePattern memory p = dao.getPattern(patternId);
        assertEq(uint256(p.scale), uint256(FractalDAO.GovernanceScale.Community));
        assertEq(p.quorumBps, 5000);
        assertEq(p.supermajorityBps, 6667);
    }

    function test_createPattern_revertsDuplicate() public {
        bytes32 patternId = keccak256("pattern-2");
        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Team,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Consent
        );

        vm.expectRevert("FractalDAO: pattern exists");
        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Team,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Consent
        );
    }

    // =========================================================================
    // Structural Identity (Fractal Invariant)
    // =========================================================================

    function test_replicateToChild_preservesStructure() public {
        bytes32 parentId = keccak256("parent");
        bytes32 childId = keccak256("child");

        dao.createPattern(
            parentId,
            FractalDAO.GovernanceScale.Regional,
            5000,
            7500,
            FractalDAO.DecisionMechanism.Supermajority
        );

        dao.replicateToChildScale(parentId, childId);

        FractalDAO.GovernancePattern memory parent = dao.getPattern(parentId);
        FractalDAO.GovernancePattern memory child = dao.getPattern(childId);

        // KEY INVARIANT: Structural identity preserved across scales
        assertEq(child.quorumBps, parent.quorumBps);
        assertEq(child.supermajorityBps, parent.supermajorityBps);
        assertEq(uint256(child.mechanism), uint256(parent.mechanism));

        // Scale is one level down
        assertEq(uint256(child.scale), uint256(parent.scale) - 1);
    }

    function test_verifyStructuralIdentity_trueForReplicated() public {
        bytes32 parentId = keccak256("parent-2");
        bytes32 childId = keccak256("child-2");

        dao.createPattern(
            parentId,
            FractalDAO.GovernanceScale.Community,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Consensus
        );
        dao.replicateToChildScale(parentId, childId);

        assertTrue(dao.verifyStructuralIdentity(parentId, childId));
    }

    function test_verifyStructuralIdentity_falseForDifferent() public {
        bytes32 id1 = keccak256("p1");
        bytes32 id2 = keccak256("p2");

        dao.createPattern(id1, FractalDAO.GovernanceScale.Team, 5000, 6667, FractalDAO.DecisionMechanism.Consent);
        dao.createPattern(id2, FractalDAO.GovernanceScale.Team, 3000, 8000, FractalDAO.DecisionMechanism.Supermajority);

        assertFalse(dao.verifyStructuralIdentity(id1, id2));
    }

    function test_cannotReplicateBelowIndividual() public {
        bytes32 individualId = keccak256("individual");
        dao.createPattern(
            individualId,
            FractalDAO.GovernanceScale.Individual,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Consent
        );

        vm.expectRevert("FractalDAO: cannot go below Individual");
        dao.replicateToChildScale(individualId, keccak256("below"));
    }

    // =========================================================================
    // Proposal & Voting
    // =========================================================================

    function test_createAndResolveProposal() public {
        bytes32 patternId = keccak256("gov-pattern");
        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Community,
            5000, // 50% quorum
            6667, // 66.67% supermajority
            FractalDAO.DecisionMechanism.Supermajority
        );

        bytes32 proposalId = keccak256("proposal-1");
        dao.createProposal(proposalId, patternId, "Test proposal", 3, 1 hours);

        // Vote
        vm.prank(voter1);
        dao.vote(proposalId, true);
        vm.prank(voter2);
        dao.vote(proposalId, true);
        vm.prank(voter3);
        dao.vote(proposalId, false);

        // Resolve after deadline
        vm.warp(block.timestamp + 2 hours);
        dao.resolveProposal(proposalId);

        FractalDAO.Proposal memory p = dao.getProposal(proposalId);
        // 2 for, 1 against. Total 3/3 = 100% quorum (>50%).
        // 2/3 = 66.67% >= 66.67% supermajority => Passed
        assertEq(uint256(p.state), uint256(FractalDAO.ProposalState.Passed));
    }

    function test_proposalRejected_insufficientQuorum() public {
        bytes32 patternId = keccak256("gov-pattern-2");
        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Community,
            5000, // 50% quorum
            6667,
            FractalDAO.DecisionMechanism.Supermajority
        );

        bytes32 proposalId = keccak256("proposal-2");
        dao.createProposal(proposalId, patternId, "Low turnout", 10, 1 hours);

        // Only 1 of 10 eligible voters votes
        vm.prank(voter1);
        dao.vote(proposalId, true);

        vm.warp(block.timestamp + 2 hours);
        dao.resolveProposal(proposalId);

        FractalDAO.Proposal memory p = dao.getProposal(proposalId);
        assertEq(uint256(p.state), uint256(FractalDAO.ProposalState.Rejected));
    }

    // =========================================================================
    // Fuzz Tests
    // =========================================================================

    /**
     * @notice Fuzz test: pattern IDs must be unique
     * @dev Tests that duplicate pattern IDs are rejected
     * @param id1 First pattern ID
     * @param id2 Second pattern ID (may or may not equal id1)
     */
    function testFuzz_createPattern_uniqueIds(bytes32 id1, bytes32 id2) public {
        // Assume both IDs are non-zero (valid)
        vm.assume(id1 != bytes32(0));
        vm.assume(id2 != bytes32(0));

        // Create first pattern
        dao.createPattern(
            id1,
            FractalDAO.GovernanceScale.Team,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Consent
        );

        // Verify first pattern exists
        FractalDAO.GovernancePattern memory p1 = dao.getPattern(id1);
        assertTrue(p1.exists, "First pattern must exist");

        if (id1 == id2) {
            // Same ID: should revert as duplicate
            vm.expectRevert("FractalDAO: pattern exists");
            dao.createPattern(
                id2,
                FractalDAO.GovernanceScale.Team,
                5000,
                6667,
                FractalDAO.DecisionMechanism.Consent
            );
        } else {
            // Different ID: should succeed
            dao.createPattern(
                id2,
                FractalDAO.GovernanceScale.Community,
                6000,
                7500,
                FractalDAO.DecisionMechanism.Supermajority
            );

            FractalDAO.GovernancePattern memory p2 = dao.getPattern(id2);
            assertTrue(p2.exists, "Second pattern must exist");

            // Verify patterns are distinct
            assertTrue(p1.patternId != p2.patternId || id1 == id2, "Pattern IDs must be unique");
        }
    }

    /**
     * @notice Fuzz test: replicate preserves structural identity across levels
     * @dev KEY INVARIANT: Governance patterns are structurally identical at all scales
     * @param levels Number of levels to replicate down (1-5)
     */
    function testFuzz_replicate_preservesStructure(uint8 levels) public {
        // Bound levels to valid range (1-5, since Global=5 can go down to Individual=0)
        vm.assume(levels >= 1 && levels <= 5);

        // Start at Global scale (5) to allow maximum replication depth
        bytes32 rootId = keccak256(abi.encodePacked("fuzz-root", levels));
        uint256 quorumBps = 5500;
        uint256 supermajorityBps = 7000;
        FractalDAO.DecisionMechanism mechanism = FractalDAO.DecisionMechanism.Consensus;

        dao.createPattern(
            rootId,
            FractalDAO.GovernanceScale.Global,
            quorumBps,
            supermajorityBps,
            mechanism
        );

        FractalDAO.GovernancePattern memory root = dao.getPattern(rootId);
        bytes32 currentParentId = rootId;
        uint8 currentScale = 5; // Global

        // Replicate down through levels
        for (uint8 i = 0; i < levels && currentScale > 0; i++) {
            bytes32 childId = keccak256(abi.encodePacked("fuzz-child", levels, i));

            dao.replicateToChildScale(currentParentId, childId);

            FractalDAO.GovernancePattern memory child = dao.getPattern(childId);

            // KEY INVARIANT: Structural identity must be preserved
            assertEq(
                child.quorumBps,
                root.quorumBps,
                "Quorum must be identical across scales"
            );
            assertEq(
                child.supermajorityBps,
                root.supermajorityBps,
                "Supermajority must be identical across scales"
            );
            assertEq(
                uint256(child.mechanism),
                uint256(root.mechanism),
                "Decision mechanism must be identical across scales"
            );

            // Verify scale decreased by 1
            assertEq(
                uint256(child.scale),
                currentScale - 1,
                "Scale must decrease by 1 on replication"
            );

            // Verify structural identity function
            assertTrue(
                dao.verifyStructuralIdentity(rootId, childId),
                "Structural identity verification must pass"
            );

            currentParentId = childId;
            currentScale--;
        }
    }

    /**
     * @notice Fuzz test: quorum enforcement in proposal resolution
     * @dev Tests that quorum logic correctly determines proposal outcomes
     * @param votesFor Number of votes in favor
     * @param votesAgainst Number of votes against
     * @param totalEligible Total eligible voters
     * @param quorumBps Quorum requirement in basis points
     * @param supermajorityBps Supermajority requirement in basis points
     */
    function testFuzz_proposal_quorumEnforcement(
        uint256 votesFor,
        uint256 votesAgainst,
        uint256 totalEligible,
        uint256 quorumBps,
        uint256 supermajorityBps
    ) public {
        // Bound inputs to reasonable ranges
        totalEligible = bound(totalEligible, 1, 1000);
        votesFor = bound(votesFor, 0, totalEligible);
        votesAgainst = bound(votesAgainst, 0, totalEligible - votesFor);
        quorumBps = bound(quorumBps, 1, 10000);
        supermajorityBps = bound(supermajorityBps, 5000, 10000);

        // Skip if no votes (edge case)
        vm.assume(votesFor + votesAgainst > 0 || quorumBps > 0);

        bytes32 patternId = keccak256(abi.encodePacked("fuzz-pattern", quorumBps, supermajorityBps));
        bytes32 proposalId = keccak256(abi.encodePacked("fuzz-proposal", votesFor, votesAgainst, totalEligible));

        // Create governance pattern
        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Community,
            quorumBps,
            supermajorityBps,
            FractalDAO.DecisionMechanism.Supermajority
        );

        // Create proposal
        dao.createProposal(proposalId, patternId, "Fuzz test proposal", totalEligible, 1 hours);

        // Cast votes using unique voter addresses
        for (uint256 i = 0; i < votesFor; i++) {
            address voter = address(uint160(0x1000 + i));
            vm.prank(voter);
            dao.vote(proposalId, true);
        }

        for (uint256 i = 0; i < votesAgainst; i++) {
            address voter = address(uint160(0x2000 + i));
            vm.prank(voter);
            dao.vote(proposalId, false);
        }

        // Move past deadline and resolve
        vm.warp(block.timestamp + 2 hours);
        dao.resolveProposal(proposalId);

        // Calculate expected outcome
        uint256 totalVotes = votesFor + votesAgainst;
        uint256 quorumRequired = (totalEligible * quorumBps) / 10000;
        uint256 supermajorityRequired = (totalVotes * supermajorityBps) / 10000;

        FractalDAO.Proposal memory p = dao.getProposal(proposalId);

        // Verify correct outcome based on quorum and supermajority
        if (totalVotes < quorumRequired) {
            // Quorum not met -> Rejected
            assertEq(
                uint256(p.state),
                uint256(FractalDAO.ProposalState.Rejected),
                "Proposal must be rejected when quorum not met"
            );
        } else if (votesFor >= supermajorityRequired) {
            // Quorum met AND supermajority achieved -> Passed
            assertEq(
                uint256(p.state),
                uint256(FractalDAO.ProposalState.Passed),
                "Proposal must pass when quorum met and supermajority achieved"
            );
        } else {
            // Quorum met but supermajority NOT achieved -> Rejected
            assertEq(
                uint256(p.state),
                uint256(FractalDAO.ProposalState.Rejected),
                "Proposal must be rejected when supermajority not achieved"
            );
        }
    }

    /**
     * @notice Fuzz test: cannot replicate below Individual scale
     * @dev Tests the scale boundary enforcement
     * @param scale Starting scale (will test Individual specifically)
     */
    function testFuzz_replicate_cannotGoBelowIndividual(uint8 scale) public {
        // Only Individual scale (0) should fail to replicate down
        vm.assume(scale <= 5);

        bytes32 parentId = keccak256(abi.encodePacked("fuzz-boundary", scale));
        bytes32 childId = keccak256(abi.encodePacked("fuzz-boundary-child", scale));

        dao.createPattern(
            parentId,
            FractalDAO.GovernanceScale(scale),
            5000,
            6667,
            FractalDAO.DecisionMechanism.Consent
        );

        if (scale == 0) {
            // Individual scale cannot replicate down
            vm.expectRevert("FractalDAO: cannot go below Individual");
            dao.replicateToChildScale(parentId, childId);
        } else {
            // All other scales can replicate down
            dao.replicateToChildScale(parentId, childId);

            FractalDAO.GovernancePattern memory child = dao.getPattern(childId);
            assertEq(uint256(child.scale), scale - 1);
        }
    }

    /**
     * @notice Fuzz test: quorum and supermajority bounds are enforced
     * @param quorumBps Attempted quorum value
     * @param supermajorityBps Attempted supermajority value
     */
    function testFuzz_createPattern_validParameters(
        uint256 quorumBps,
        uint256 supermajorityBps
    ) public {
        bytes32 patternId = keccak256(abi.encodePacked("fuzz-params", quorumBps, supermajorityBps));

        // Quorum must be 1-10000, supermajority must be 5000-10000
        bool validQuorum = quorumBps > 0 && quorumBps <= 10000;
        bool validSupermajority = supermajorityBps >= 5000 && supermajorityBps <= 10000;

        if (!validQuorum) {
            vm.expectRevert("FractalDAO: invalid quorum");
            dao.createPattern(
                patternId,
                FractalDAO.GovernanceScale.Team,
                quorumBps,
                supermajorityBps,
                FractalDAO.DecisionMechanism.Consent
            );
        } else if (!validSupermajority) {
            vm.expectRevert("FractalDAO: invalid supermajority");
            dao.createPattern(
                patternId,
                FractalDAO.GovernanceScale.Team,
                quorumBps,
                supermajorityBps,
                FractalDAO.DecisionMechanism.Consent
            );
        } else {
            // Both valid
            dao.createPattern(
                patternId,
                FractalDAO.GovernanceScale.Team,
                quorumBps,
                supermajorityBps,
                FractalDAO.DecisionMechanism.Consent
            );

            FractalDAO.GovernancePattern memory p = dao.getPattern(patternId);
            assertEq(p.quorumBps, quorumBps);
            assertEq(p.supermajorityBps, supermajorityBps);
        }
    }

    /**
     * @notice Fuzz test: double voting is prevented
     * @param voterSeed Seed to generate voter address
     */
    function testFuzz_vote_preventDoubleVoting(uint256 voterSeed) public {
        address voter = address(uint160(bound(voterSeed, 1, type(uint160).max)));

        bytes32 patternId = keccak256("fuzz-double-vote-pattern");
        bytes32 proposalId = keccak256(abi.encodePacked("fuzz-double-vote", voterSeed));

        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Team,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Consent
        );

        dao.createProposal(proposalId, patternId, "Test double vote", 100, 1 hours);

        // First vote should succeed
        vm.prank(voter);
        dao.vote(proposalId, true);

        // Second vote should revert
        vm.prank(voter);
        vm.expectRevert("FractalDAO: already voted");
        dao.vote(proposalId, true);

        // Even voting the opposite way should fail
        vm.prank(voter);
        vm.expectRevert("FractalDAO: already voted");
        dao.vote(proposalId, false);
    }
}
