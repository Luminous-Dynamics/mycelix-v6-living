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
        require(releaseBps > 0, "KenosisBurn: zero release");
        require(releaseBps <= MAX_RELEASE_BPS, "KenosisBurn: exceeds 20% cap per cycle");
        require(!commitments[commitmentId].irrevocable, "KenosisBurn: commitment exists");

        // Check cycle cap
        uint256 cycleTotal = cycleReleases[msg.sender][currentCycle] + releaseBps;
        require(cycleTotal <= MAX_RELEASE_BPS, "KenosisBurn: cycle cap exceeded");

        // Get current reputation balance
        uint256 balance = reputationToken.balanceOf(msg.sender);
        require(balance > 0, "KenosisBurn: no reputation to release");

        uint256 burnAmount = (balance * releaseBps) / BPS_DENOMINATOR;
        require(burnAmount > 0, "KenosisBurn: burn amount rounds to zero");

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
        require(commitment.irrevocable, "KenosisBurn: commitment not found");
        require(!commitment.executed, "KenosisBurn: already executed");
        require(commitment.agent == msg.sender, "KenosisBurn: not your commitment");

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
        require(commitments[commitmentId].irrevocable, "KenosisBurn: not found");
        return commitments[commitmentId];
    }

    function getAgentCommitments(address agent) external view returns (bytes32[] memory) {
        return agentCommitments[agent];
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
