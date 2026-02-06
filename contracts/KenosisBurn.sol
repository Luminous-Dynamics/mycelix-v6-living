// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

/**
 * @title KenosisBurn
 * @notice On-chain kenosis (self-emptying) mechanism for voluntary reputation release [Primitive 4].
 * @dev Implements the "strange loop" anti-gaming property: gaming kenosis (strategic
 *      self-emptying for social capital) IS genuine kenosis. The game-theoretic
 *      exploit collapses into the authentic act.
 *
 * KEY INVARIANTS:
 *   1. Kenosis is IRREVOCABLE once committed
 *   2. Maximum 20% reputation release per cycle
 *   3. Released reputation is permanently burned (not redistributed)
 *
 * Constitutional Alignment: Evolutionary Progression (Harmony 7)
 */

import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "./libraries/Errors.sol";

contract KenosisBurn is AccessControl, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // =========================================================================
    // Roles
    // =========================================================================

    bytes32 public constant CYCLE_MANAGER_ROLE = keccak256("CYCLE_MANAGER_ROLE");

    // =========================================================================
    // Constants
    // =========================================================================

    /// @notice Maximum release percentage per cycle (20% = 2000 basis points)
    uint256 public constant MAX_RELEASE_BPS = 2000;
    uint256 public constant BPS_DENOMINATOR = 10000;

    // =========================================================================
    // Types
    // =========================================================================

    struct KenosisCommitment {
        bytes32 commitmentId;
        address agent;
        uint256 releaseBps;       // Basis points of reputation to release
        uint256 reputationBurned; // Actual amount burned
        uint256 cycleNumber;
        uint256 committedAt;
        bool executed;
        bool irrevocable;         // Always true once committed
    }

    // =========================================================================
    // State
    // =========================================================================

    IERC20 public immutable reputationToken;
    uint256 public currentCycle;

    mapping(bytes32 => KenosisCommitment) public commitments;
    mapping(address => mapping(uint256 => uint256)) public cycleReleases; // agent => cycle => total bps
    mapping(address => bytes32[]) public agentCommitments;

    uint256 public totalBurned;

    // =========================================================================
    // Events
    // =========================================================================

    event KenosisCommitted(
        bytes32 indexed commitmentId,
        address indexed agent,
        uint256 releaseBps,
        uint256 cycleNumber,
        uint256 timestamp
    );

    event KenosisExecuted(
        bytes32 indexed commitmentId,
        address indexed agent,
        uint256 reputationBefore,
        uint256 reputationAfter,
        uint256 burned,
        uint256 timestamp
    );

    event CycleAdvanced(
        uint256 oldCycle,
        uint256 newCycle,
        uint256 timestamp
    );

    // =========================================================================
    // Constructor
    // =========================================================================

    constructor(address _reputationToken) {
        reputationToken = IERC20(_reputationToken);
        currentCycle = 1;
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(CYCLE_MANAGER_ROLE, msg.sender);
    }

    // =========================================================================
    // Core Functions
    // =========================================================================

    /**
     * @notice Commit to a kenosis (self-emptying) of reputation.
     * @dev Once committed, this is IRREVOCABLE. The strange loop property means
     *      that even strategic commitment is genuine kenosis.
     * @param commitmentId Unique identifier for this commitment
     * @param releaseBps Basis points of reputation to release (max 2000 = 20%)
     */
    function commitKenosis(
        bytes32 commitmentId,
        uint256 releaseBps
    ) external nonReentrant {
        if (releaseBps == 0) revert ZeroRelease();
        if (releaseBps > MAX_RELEASE_BPS) revert ExceedsMaxRelease(releaseBps, MAX_RELEASE_BPS);
        if (commitments[commitmentId].irrevocable) revert CommitmentAlreadyExists(commitmentId);

        // Check cycle cap
        uint256 currentUsed = cycleReleases[msg.sender][currentCycle];
        uint256 cycleTotal = currentUsed + releaseBps;
        if (cycleTotal > MAX_RELEASE_BPS) {
            revert CycleCapExceeded(msg.sender, currentCycle, currentUsed, releaseBps, MAX_RELEASE_BPS);
        }

        // Get current reputation balance
        uint256 balance = reputationToken.balanceOf(msg.sender);
        if (balance == 0) revert NoReputationToRelease(msg.sender);

        uint256 burnAmount = (balance * releaseBps) / BPS_DENOMINATOR;
        if (burnAmount == 0) revert BurnAmountRoundsToZero();

        // Record commitment (IRREVOCABLE)
        commitments[commitmentId] = KenosisCommitment({
            commitmentId: commitmentId,
            agent: msg.sender,
            releaseBps: releaseBps,
            reputationBurned: burnAmount,
            cycleNumber: currentCycle,
            committedAt: block.timestamp,
            executed: false,
            irrevocable: true  // Once set, cannot be undone
        });

        cycleReleases[msg.sender][currentCycle] += releaseBps;
        agentCommitments[msg.sender].push(commitmentId);

        emit KenosisCommitted(commitmentId, msg.sender, releaseBps, currentCycle, block.timestamp);
    }

    /**
     * @notice Execute a kenosis commitment by burning the reputation tokens.
     * @dev Can only be called after commitment. Burns tokens permanently.
     */
    function executeKenosis(bytes32 commitmentId) external nonReentrant {
        KenosisCommitment storage commitment = commitments[commitmentId];
        if (!commitment.irrevocable) revert CommitmentNotFound(commitmentId);
        if (commitment.executed) revert CommitmentAlreadyExecuted(commitmentId);
        if (commitment.agent != msg.sender) revert NotCommitmentOwner(commitmentId, msg.sender, commitment.agent);

        uint256 balanceBefore = reputationToken.balanceOf(msg.sender);
        uint256 burnAmount = commitment.reputationBurned;

        // Transfer to this contract, then burn
        reputationToken.safeTransferFrom(msg.sender, address(this), burnAmount);

        // Burn the tokens (send to dead address if not burnable)
        try ERC20Burnable(address(reputationToken)).burn(burnAmount) {
            // Successfully burned via ERC20Burnable
        } catch {
            // Fallback: send to dead address
            reputationToken.safeTransfer(address(0xdead), burnAmount);
        }

        commitment.executed = true;
        totalBurned += burnAmount;

        uint256 balanceAfter = reputationToken.balanceOf(msg.sender);

        emit KenosisExecuted(
            commitmentId,
            msg.sender,
            balanceBefore,
            balanceAfter,
            burnAmount,
            block.timestamp
        );
    }

    /**
     * @notice Advance to the next cycle.
     */
    function advanceCycle() external onlyRole(CYCLE_MANAGER_ROLE) {
        uint256 oldCycle = currentCycle;
        currentCycle += 1;
        emit CycleAdvanced(oldCycle, currentCycle, block.timestamp);
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    function getCommitment(bytes32 commitmentId) external view returns (KenosisCommitment memory) {
        if (!commitments[commitmentId].irrevocable) revert CommitmentNotFound(commitmentId);
        return commitments[commitmentId];
    }

    function getAgentCommitments(address agent) external view returns (bytes32[] memory) {
        return agentCommitments[agent];
    }

    /**
     * @notice Get commitments for an agent with pagination.
     * @dev Prevents gas issues with agents having many commitments.
     * @param agent The agent's address
     * @param offset Starting index
     * @param limit Maximum number of commitments to return
     * @return commitmentIds Array of commitment IDs
     * @return hasMore True if there are more commitments beyond this page
     */
    function getAgentCommitmentsPaginated(
        address agent,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory commitmentIds, bool hasMore) {
        bytes32[] storage all = agentCommitments[agent];
        uint256 total = all.length;

        if (offset >= total) {
            return (new bytes32[](0), false);
        }

        uint256 end = offset + limit;
        if (end > total) {
            end = total;
        }

        commitmentIds = new bytes32[](end - offset);
        for (uint256 i = offset; i < end; i++) {
            commitmentIds[i - offset] = all[i];
        }

        hasMore = end < total;
    }

    function getCycleReleaseTotal(address agent, uint256 cycle) external view returns (uint256) {
        return cycleReleases[agent][cycle];
    }

    function getRemainingCycleCapacity(address agent) external view returns (uint256) {
        uint256 used = cycleReleases[agent][currentCycle];
        if (used >= MAX_RELEASE_BPS) return 0;
        return MAX_RELEASE_BPS - used;
    }

    function isIrrevocable(bytes32 commitmentId) external view returns (bool) {
        return commitments[commitmentId].irrevocable;
    }

    function getTotalBurned() external view returns (uint256) {
        return totalBurned;
    }
}
