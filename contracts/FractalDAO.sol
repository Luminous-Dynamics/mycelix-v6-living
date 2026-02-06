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
import "./libraries/Errors.sol";

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
        if (patterns[patternId].exists) revert PatternAlreadyExists(patternId);
        if (quorumBps == 0 || quorumBps > 10000) revert InvalidQuorum(quorumBps);
        if (supermajorityBps < 5000 || supermajorityBps > 10000) revert InvalidSupermajority(supermajorityBps);

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
        if (!parent.exists) revert PatternNotFound(parentId);
        if (patterns[childId].exists) revert PatternAlreadyExists(childId);
        if (uint8(parent.scale) == 0) revert CannotScaleBelowIndividual(parentId);

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
        if (!child.exists) revert PatternNotFound(childId);
        if (patterns[parentId].exists) revert PatternAlreadyExists(parentId);
        if (uint8(child.scale) >= 5) revert CannotScaleAboveGlobal(childId);

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
        if (!patterns[patternA].exists) revert PatternNotFound(patternA);
        if (!patterns[patternB].exists) revert PatternNotFound(patternB);

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
        if (!patterns[patternId].exists) revert PatternNotFound(patternId);
        if (proposals[proposalId].exists) revert ProposalAlreadyExists(proposalId);
        if (eligibleVoters == 0) revert NoEligibleVoters(proposalId);

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
        if (!proposal.exists) revert ProposalNotFound(proposalId);
        if (proposal.state != ProposalState.Active) revert ProposalNotActive(proposalId, uint8(proposal.state));
        if (block.timestamp > proposal.deadline) revert VotingEnded(proposalId, proposal.deadline, block.timestamp);
        if (hasVoted[proposalId][msg.sender]) revert AlreadyVoted(proposalId, msg.sender);

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
        if (!proposal.exists) revert ProposalNotFound(proposalId);
        if (proposal.state != ProposalState.Active) revert ProposalNotActive(proposalId, uint8(proposal.state));
        if (block.timestamp <= proposal.deadline) revert VotingNotEnded(proposalId, proposal.deadline, block.timestamp);

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
        if (!patterns[patternId].exists) revert PatternNotFound(patternId);
        return patterns[patternId];
    }

    function getChildPatterns(bytes32 patternId) external view returns (bytes32[] memory) {
        return childPatterns[patternId];
    }

    function getProposal(bytes32 proposalId) external view returns (Proposal memory) {
        if (!proposals[proposalId].exists) revert ProposalNotFound(proposalId);
        return proposals[proposalId];
    }

    function getAllPatterns() external view returns (bytes32[] memory) {
        return allPatterns;
    }

    /**
     * @notice Get all patterns with pagination.
     * @dev Prevents gas issues with many patterns.
     * @param offset Starting index
     * @param limit Maximum number of patterns to return
     * @return patternIds Array of pattern IDs
     * @return hasMore True if there are more patterns beyond this page
     */
    function getAllPatternsPaginated(
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory patternIds, bool hasMore) {
        uint256 total = allPatterns.length;

        if (offset >= total) {
            return (new bytes32[](0), false);
        }

        uint256 end = offset + limit;
        if (end > total) {
            end = total;
        }

        patternIds = new bytes32[](end - offset);
        for (uint256 i = offset; i < end; i++) {
            patternIds[i - offset] = allPatterns[i];
        }

        hasMore = end < total;
    }

    /**
     * @notice Get child patterns with pagination.
     * @dev Prevents gas issues with patterns having many children.
     * @param patternId The parent pattern ID
     * @param offset Starting index
     * @param limit Maximum number of children to return
     * @return children Array of child pattern IDs
     * @return hasMore True if there are more children beyond this page
     */
    function getChildPatternsPaginated(
        bytes32 patternId,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory children, bool hasMore) {
        bytes32[] storage all = childPatterns[patternId];
        uint256 total = all.length;

        if (offset >= total) {
            return (new bytes32[](0), false);
        }

        uint256 end = offset + limit;
        if (end > total) {
            end = total;
        }

        children = new bytes32[](end - offset);
        for (uint256 i = offset; i < end; i++) {
            children[i - offset] = all[i];
        }

        hasMore = end < total;
    }
}
