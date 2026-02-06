// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

/**
 * @title Errors
 * @notice Custom error definitions for gas-efficient reverts across Mycelix contracts.
 * @dev Custom errors save ~2000 gas per revert compared to require strings.
 */

// =============================================================================
// WoundEscrow Errors
// =============================================================================

/// @notice Wound with this ID already exists.
error WoundAlreadyExists(bytes32 woundId);

/// @notice Wound with this ID was not found.
error WoundNotFound(bytes32 woundId);

/// @notice Cannot perform operation on an agent with zero address.
error ZeroAddress();

/// @notice Escrow amount must be greater than zero.
error ZeroEscrow();

/// @notice Invalid phase transition attempted.
error InvalidPhaseTransition(uint8 fromPhase, uint8 toPhase);

/// @notice Wound has already completed healing.
error WoundAlreadyHealed(bytes32 woundId);

/// @notice Minimum hemostasis duration has not elapsed.
error MinHemostasisNotElapsed(bytes32 woundId, uint256 required, uint256 elapsed);

/// @notice Restitution has not been fully paid.
error RestitutionNotFulfilled(bytes32 woundId, uint256 required, uint256 paid);

/// @notice Operation not allowed in current wound phase.
error WrongPhase(bytes32 woundId, uint8 currentPhase, uint8 requiredPhase);

/// @notice Payment amount must be greater than zero.
error ZeroPayment();

/// @notice Scar tissue multiplier out of valid range.
error InvalidScarMultiplier(uint256 multiplier, uint256 minAllowed, uint256 maxAllowed);

// =============================================================================
// KenosisBurn Errors
// =============================================================================

/// @notice Release amount is zero.
error ZeroRelease();

/// @notice Release percentage exceeds maximum allowed per cycle (20%).
error ExceedsMaxRelease(uint256 requestedBps, uint256 maxBps);

/// @notice Commitment with this ID already exists.
error CommitmentAlreadyExists(bytes32 commitmentId);

/// @notice Total release for this cycle would exceed the 20% cap.
error CycleCapExceeded(address agent, uint256 cycle, uint256 currentBps, uint256 requestedBps, uint256 maxBps);

/// @notice Agent has no reputation to release.
error NoReputationToRelease(address agent);

/// @notice Burn amount rounds to zero due to small balance or percentage.
error BurnAmountRoundsToZero();

/// @notice Commitment was not found.
error CommitmentNotFound(bytes32 commitmentId);

/// @notice Commitment has already been executed.
error CommitmentAlreadyExecuted(bytes32 commitmentId);

/// @notice Only the committing agent can execute their commitment.
error NotCommitmentOwner(bytes32 commitmentId, address caller, address owner);

// =============================================================================
// FractalDAO Errors
// =============================================================================

/// @notice Pattern with this ID already exists.
error PatternAlreadyExists(bytes32 patternId);

/// @notice Pattern with this ID was not found.
error PatternNotFound(bytes32 patternId);

/// @notice Quorum value is invalid (must be > 0 and <= 10000 bps).
error InvalidQuorum(uint256 quorumBps);

/// @notice Supermajority value is invalid (must be >= 5000 and <= 10000 bps).
error InvalidSupermajority(uint256 supermajorityBps);

/// @notice Cannot create child pattern below Individual scale.
error CannotScaleBelowIndividual(bytes32 patternId);

/// @notice Cannot create parent pattern above Global scale.
error CannotScaleAboveGlobal(bytes32 patternId);

/// @notice Proposal with this ID already exists.
error ProposalAlreadyExists(bytes32 proposalId);

/// @notice Proposal with this ID was not found.
error ProposalNotFound(bytes32 proposalId);

/// @notice Proposal has no eligible voters.
error NoEligibleVoters(bytes32 proposalId);

/// @notice Proposal is not in active state.
error ProposalNotActive(bytes32 proposalId, uint8 currentState);

/// @notice Voting period has ended.
error VotingEnded(bytes32 proposalId, uint256 deadline, uint256 currentTime);

/// @notice Voting period has not ended yet.
error VotingNotEnded(bytes32 proposalId, uint256 deadline, uint256 currentTime);

/// @notice Voter has already voted on this proposal.
error AlreadyVoted(bytes32 proposalId, address voter);
