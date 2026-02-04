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
}
