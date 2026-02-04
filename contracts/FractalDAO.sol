// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

/**
 * @title FractalDAO
 * @notice Self-similar governance at all scales [Primitive 18].
 * @dev Implements fractal governance patterns that are structurally identical
 *      at every scale (Individual → Team → Community → Sector → Regional → Global).
 *
 * KEY INVARIANT: Governance patterns are structurally identical at all scales.
 * The same quorum ratios, supermajority requirements, and decision mechanisms
 * apply regardless of scale.
 *
 * Constitutional Alignment: Resonant Coherence (Harmony 1), Subsidiarity
 */

import "@openzeppelin/contracts/access/AccessControl.sol";

contract FractalDAO is AccessControl {
    // =========================================================================
    // Types
    // =========================================================================

    enum GovernanceScale {
        Individual,  // 0
        Team,        // 1
        Community,   // 2
        Sector,      // 3
        Regional,    // 4
        Global       // 5
    }

    enum DecisionMechanism {
        Consent,           // 0
        Consensus,         // 1
        Supermajority,     // 2
        ReputationWeighted // 3
    }

    enum ProposalState {
        Pending,
        Active,
        Passed,
        Rejected,
        Executed
    }

    struct GovernancePattern {
        bytes32 patternId;
        GovernanceScale scale;
        bytes32 parentPatternId;
        uint256 quorumBps;         // Quorum in basis points (e.g., 5000 = 50%)
        uint256 supermajorityBps;  // Supermajority in basis points (e.g., 6667 = 66.67%)
        DecisionMechanism mechanism;
        uint256 createdAt;
        bool exists;
    }

    struct Proposal {
        bytes32 proposalId;
        bytes32 patternId;
        address proposer;
        string description;
        ProposalState state;
        uint256 votesFor;
        uint256 votesAgainst;
        uint256 totalEligibleVoters;
        uint256 createdAt;
        uint256 deadline;
        bool exists;
    }

    // =========================================================================
    // State
    // =========================================================================

    mapping(bytes32 => GovernancePattern) public patterns;
    mapping(bytes32 => bytes32[]) public childPatterns;
    mapping(bytes32 => Proposal) public proposals;
    mapping(bytes32 => mapping(address => bool)) public hasVoted;

    bytes32[] public allPatterns;
    uint256 public patternCount;

    // =========================================================================
    // Events
    // =========================================================================

    event PatternCreated(
        bytes32 indexed patternId,
        GovernanceScale scale,
        bytes32 parentPatternId,
        uint256 quorumBps,
        uint256 supermajorityBps,
        DecisionMechanism mechanism
    );

    event PatternReplicated(
        bytes32 indexed parentId,
        bytes32 indexed childId,
        GovernanceScale parentScale,
        GovernanceScale childScale
    );

    event ProposalCreated(
        bytes32 indexed proposalId,
        bytes32 indexed patternId,
        address proposer,
        string description
    );

    event Voted(
        bytes32 indexed proposalId,
        address indexed voter,
        bool support
    );

    event ProposalResolved(
        bytes32 indexed proposalId,
        ProposalState state
    );

    event StructuralIdentityVerified(
        bytes32 indexed patternA,
        bytes32 indexed patternB,
        bool identical
    );

    // =========================================================================
    // Constructor
    // =========================================================================

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    // =========================================================================
    // Pattern Management
    // =========================================================================

    /**
     * @notice Create a new governance pattern at a given scale.
     */
    function createPattern(
        bytes32 patternId,
        GovernanceScale scale,
        uint256 quorumBps,
        uint256 supermajorityBps,
        DecisionMechanism mechanism
    ) external returns (bytes32) {
        require(!patterns[patternId].exists, "FractalDAO: pattern exists");
        require(quorumBps > 0 && quorumBps <= 10000, "FractalDAO: invalid quorum");
        require(supermajorityBps >= 5000 && supermajorityBps <= 10000, "FractalDAO: invalid supermajority");

        patterns[patternId] = GovernancePattern({
            patternId: patternId,
            scale: scale,
            parentPatternId: bytes32(0),
            quorumBps: quorumBps,
            supermajorityBps: supermajorityBps,
            mechanism: mechanism,
            createdAt: block.timestamp,
            exists: true
        });

        allPatterns.push(patternId);
        patternCount++;

        emit PatternCreated(patternId, scale, bytes32(0), quorumBps, supermajorityBps, mechanism);

        return patternId;
    }

    /**
     * @notice Replicate a pattern to a child scale.
     * @dev Creates a structurally identical pattern at the next lower scale.
     *      KEY INVARIANT: The child pattern has identical quorum, supermajority,
     *      and decision mechanism as the parent.
     */
    function replicateToChildScale(
        bytes32 parentId,
        bytes32 childId
    ) external returns (bytes32) {
        GovernancePattern storage parent = patterns[parentId];
        require(parent.exists, "FractalDAO: parent not found");
        require(!patterns[childId].exists, "FractalDAO: child exists");
        require(uint8(parent.scale) > 0, "FractalDAO: cannot go below Individual");

        GovernanceScale childScale = GovernanceScale(uint8(parent.scale) - 1);

        // Create structurally identical pattern at child scale
        patterns[childId] = GovernancePattern({
            patternId: childId,
            scale: childScale,
            parentPatternId: parentId,
            quorumBps: parent.quorumBps,             // IDENTICAL
            supermajorityBps: parent.supermajorityBps, // IDENTICAL
            mechanism: parent.mechanism,               // IDENTICAL
            createdAt: block.timestamp,
            exists: true
        });

        childPatterns[parentId].push(childId);
        allPatterns.push(childId);
        patternCount++;

        emit PatternReplicated(parentId, childId, parent.scale, childScale);

        return childId;
    }

    /**
     * @notice Replicate a pattern to a parent scale.
     */
    function replicateToParentScale(
        bytes32 childId,
        bytes32 parentId
    ) external returns (bytes32) {
        GovernancePattern storage child = patterns[childId];
        require(child.exists, "FractalDAO: child not found");
        require(!patterns[parentId].exists, "FractalDAO: parent exists");
        require(uint8(child.scale) < 5, "FractalDAO: cannot go above Global");

        GovernanceScale parentScale = GovernanceScale(uint8(child.scale) + 1);

        patterns[parentId] = GovernancePattern({
            patternId: parentId,
            scale: parentScale,
            parentPatternId: bytes32(0),
            quorumBps: child.quorumBps,
            supermajorityBps: child.supermajorityBps,
            mechanism: child.mechanism,
            createdAt: block.timestamp,
            exists: true
        });

        allPatterns.push(parentId);
        patternCount++;

        emit PatternReplicated(childId, parentId, child.scale, parentScale);

        return parentId;
    }

    /**
     * @notice Verify that two patterns are structurally identical.
     * @dev Checks quorum, supermajority, and decision mechanism.
     */
    function verifyStructuralIdentity(
        bytes32 patternA,
        bytes32 patternB
    ) external returns (bool) {
        require(patterns[patternA].exists, "FractalDAO: pattern A not found");
        require(patterns[patternB].exists, "FractalDAO: pattern B not found");

        GovernancePattern storage a = patterns[patternA];
        GovernancePattern storage b = patterns[patternB];

        bool identical = (
            a.quorumBps == b.quorumBps &&
            a.supermajorityBps == b.supermajorityBps &&
            a.mechanism == b.mechanism
        );

        emit StructuralIdentityVerified(patternA, patternB, identical);

        return identical;
    }

    // =========================================================================
    // Proposal Management
    // =========================================================================

    /**
     * @notice Create a proposal under a governance pattern.
     */
    function createProposal(
        bytes32 proposalId,
        bytes32 patternId,
        string calldata description,
        uint256 eligibleVoters,
        uint256 votingDuration
    ) external {
        require(patterns[patternId].exists, "FractalDAO: pattern not found");
        require(!proposals[proposalId].exists, "FractalDAO: proposal exists");
        require(eligibleVoters > 0, "FractalDAO: no eligible voters");

        proposals[proposalId] = Proposal({
            proposalId: proposalId,
            patternId: patternId,
            proposer: msg.sender,
            description: description,
            state: ProposalState.Active,
            votesFor: 0,
            votesAgainst: 0,
            totalEligibleVoters: eligibleVoters,
            createdAt: block.timestamp,
            deadline: block.timestamp + votingDuration,
            exists: true
        });

        emit ProposalCreated(proposalId, patternId, msg.sender, description);
    }

    /**
     * @notice Vote on a proposal.
     */
    function vote(bytes32 proposalId, bool support) external {
        Proposal storage proposal = proposals[proposalId];
        require(proposal.exists, "FractalDAO: proposal not found");
        require(proposal.state == ProposalState.Active, "FractalDAO: not active");
        require(block.timestamp <= proposal.deadline, "FractalDAO: voting ended");
        require(!hasVoted[proposalId][msg.sender], "FractalDAO: already voted");

        hasVoted[proposalId][msg.sender] = true;

        if (support) {
            proposal.votesFor++;
        } else {
            proposal.votesAgainst++;
        }

        emit Voted(proposalId, msg.sender, support);
    }

    /**
     * @notice Resolve a proposal after voting ends.
     */
    function resolveProposal(bytes32 proposalId) external {
        Proposal storage proposal = proposals[proposalId];
        require(proposal.exists, "FractalDAO: proposal not found");
        require(proposal.state == ProposalState.Active, "FractalDAO: not active");
        require(block.timestamp > proposal.deadline, "FractalDAO: voting not ended");

        GovernancePattern storage pattern = patterns[proposal.patternId];
        uint256 totalVotes = proposal.votesFor + proposal.votesAgainst;

        // Check quorum
        uint256 quorumRequired = (proposal.totalEligibleVoters * pattern.quorumBps) / BPS_DENOMINATOR;
        if (totalVotes < quorumRequired) {
            proposal.state = ProposalState.Rejected;
            emit ProposalResolved(proposalId, ProposalState.Rejected);
            return;
        }

        // Check supermajority
        uint256 supermajorityRequired = (totalVotes * pattern.supermajorityBps) / BPS_DENOMINATOR;
        if (proposal.votesFor >= supermajorityRequired) {
            proposal.state = ProposalState.Passed;
        } else {
            proposal.state = ProposalState.Rejected;
        }

        emit ProposalResolved(proposalId, proposal.state);
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    uint256 private constant BPS_DENOMINATOR = 10000;

    function getPattern(bytes32 patternId) external view returns (GovernancePattern memory) {
        require(patterns[patternId].exists, "FractalDAO: not found");
        return patterns[patternId];
    }

    function getChildPatterns(bytes32 patternId) external view returns (bytes32[] memory) {
        return childPatterns[patternId];
    }

    function getProposal(bytes32 proposalId) external view returns (Proposal memory) {
        require(proposals[proposalId].exists, "FractalDAO: not found");
        return proposals[proposalId];
    }

    function getAllPatterns() external view returns (bytes32[] memory) {
        return allPatterns;
    }
}
