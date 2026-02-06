// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../../contracts/KenosisBurn.sol";
import "../../contracts/libraries/Errors.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";

/**
 * @title KenosisBurn Formal Verification Tests (Halmos)
 * @notice Symbolic execution tests for critical KenosisBurn invariants.
 * @dev Run with: halmos --contract KenosisBurnHalmosTest --solver-timeout-assertion 300
 *
 * Critical Invariants Verified:
 *   1. 20% maximum release cap per cycle per agent
 *   2. Irrevocability is permanent once committed
 *   3. Burn permanence - tokens are permanently destroyed
 *   4. Commitment uniqueness
 *
 * Constitutional Alignment: Evolutionary Progression (Harmony 7)
 * The kenosis mechanism implements "strange loop" anti-gaming:
 * gaming kenosis (strategic self-emptying for social capital) IS genuine kenosis.
 */

/// @dev Mock burnable ERC20 for symbolic testing.
contract SymbolicReputationToken is ERC20, ERC20Burnable {
    constructor() ERC20("ReputationToken", "REP") {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract KenosisBurnHalmosTest is Test {
    KenosisBurn public kenosis;
    SymbolicReputationToken public token;

    address public admin;
    address public agent;

    uint256 public constant MAX_RELEASE_BPS = 2000; // 20%
    uint256 public constant BPS_DENOMINATOR = 10000;

    function setUp() public {
        admin = address(this);
        agent = address(0xBEEF);

        token = new SymbolicReputationToken();
        kenosis = new KenosisBurn(address(token));

        // Fund agent with reputation
        token.mint(agent, 1000 ether);
        vm.prank(agent);
        token.approve(address(kenosis), type(uint256).max);
    }

    // =========================================================================
    // Invariant 1: 20% Cap Per Cycle
    // =========================================================================

    /**
     * @notice Verify that a single commitment cannot exceed 20%.
     * @dev Symbolically tests the MAX_RELEASE_BPS enforcement.
     *
     * 20% Cap Invariant:
     * No single commitment can release more than 20% of an agent's reputation
     * in a single cycle. This prevents rapid reputation dumping.
     */
    function check_max_release_single_commitment(
        bytes32 commitmentId,
        uint256 releaseBps
    ) public {
        vm.assume(releaseBps > 0);

        vm.prank(agent);

        if (releaseBps > MAX_RELEASE_BPS) {
            // Should revert for > 20% with custom error
            vm.expectRevert(abi.encodeWithSelector(ExceedsMaxRelease.selector, releaseBps, MAX_RELEASE_BPS));
            kenosis.commitKenosis(commitmentId, releaseBps);
        } else {
            // Should succeed for <= 20%
            kenosis.commitKenosis(commitmentId, releaseBps);

            KenosisBurn.KenosisCommitment memory commitment = kenosis.getCommitment(commitmentId);
            assert(commitment.releaseBps <= MAX_RELEASE_BPS);
        }
    }

    /**
     * @notice Verify that cumulative cycle releases cannot exceed 20%.
     * @dev Tests the cycle cap accumulation logic.
     *
     * Cumulative Cap Invariant:
     * The sum of all releases in a cycle cannot exceed 20%.
     * This prevents splitting releases across multiple commitments
     * to circumvent the single-commitment cap.
     */
    function check_max_release_cumulative(
        bytes32 commitmentId1,
        bytes32 commitmentId2,
        uint256 release1Bps,
        uint256 release2Bps
    ) public {
        vm.assume(release1Bps > 0 && release1Bps <= MAX_RELEASE_BPS);
        vm.assume(release2Bps > 0 && release2Bps <= MAX_RELEASE_BPS);
        vm.assume(commitmentId1 != commitmentId2);

        uint256 currentCycle = kenosis.currentCycle();

        // First commitment
        vm.prank(agent);
        kenosis.commitKenosis(commitmentId1, release1Bps);

        uint256 cycleTotal = kenosis.getCycleReleaseTotal(agent, currentCycle);
        assert(cycleTotal == release1Bps);

        // Second commitment
        vm.prank(agent);

        if (release1Bps + release2Bps > MAX_RELEASE_BPS) {
            // Should revert if cumulative > 20% with custom error
            vm.expectRevert(abi.encodeWithSelector(
                CycleCapExceeded.selector,
                agent,
                currentCycle,
                release1Bps,
                release2Bps,
                MAX_RELEASE_BPS
            ));
            kenosis.commitKenosis(commitmentId2, release2Bps);
        } else {
            // Should succeed if cumulative <= 20%
            kenosis.commitKenosis(commitmentId2, release2Bps);

            uint256 newTotal = kenosis.getCycleReleaseTotal(agent, currentCycle);
            assert(newTotal == release1Bps + release2Bps);
            assert(newTotal <= MAX_RELEASE_BPS);
        }
    }

    /**
     * @notice Verify remaining capacity calculation is correct.
     * @dev Tests the getRemainingCycleCapacity function.
     *
     * Capacity Tracking Invariant:
     * remainingCapacity = MAX_RELEASE_BPS - usedCapacity
     * This ensures agents can always check their available release quota.
     */
    function check_remaining_capacity(bytes32 commitmentId, uint256 releaseBps) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);

        uint256 capacityBefore = kenosis.getRemainingCycleCapacity(agent);
        assert(capacityBefore == MAX_RELEASE_BPS); // Full capacity initially

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        uint256 capacityAfter = kenosis.getRemainingCycleCapacity(agent);

        // INVARIANT: Remaining capacity = MAX - used
        assert(capacityAfter == MAX_RELEASE_BPS - releaseBps);
    }

    // =========================================================================
    // Invariant 2: Irrevocability is Permanent
    // =========================================================================

    /**
     * @notice Verify that once committed, irrevocable flag is always true.
     * @dev No code path should ever set irrevocable to false.
     *
     * Irrevocability Permanence Invariant:
     * Once irrevocable is set to true, it can NEVER be set back to false.
     * This is the core anti-gaming mechanism: once you commit to kenosis,
     * there is no way to back out. This aligns with Harmony 7's principle
     * that genuine transformation requires irreversible commitment.
     */
    function check_irrevocable_permanent(bytes32 commitmentId, uint256 releaseBps) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);

        // Before commitment, should not exist (irrevocable = false)
        assert(!kenosis.isIrrevocable(commitmentId));

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        // CRITICAL INVARIANT: Once committed, irrevocable is permanently true
        assert(kenosis.isIrrevocable(commitmentId));

        // Even after execution, still irrevocable
        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        // INVARIANT: Irrevocability persists after execution
        assert(kenosis.isIrrevocable(commitmentId));
    }

    /**
     * @notice Verify commitment state consistency after creation.
     * @dev Tests all commitment fields are properly initialized.
     */
    function check_commitment_state_consistency(
        bytes32 commitmentId,
        uint256 releaseBps
    ) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);

        uint256 agentBalance = token.balanceOf(agent);
        uint256 expectedBurn = (agentBalance * releaseBps) / BPS_DENOMINATOR;
        vm.assume(expectedBurn > 0); // Ensure non-zero burn

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        KenosisBurn.KenosisCommitment memory commitment = kenosis.getCommitment(commitmentId);

        // INVARIANTS: All fields properly set
        assert(commitment.commitmentId == commitmentId);
        assert(commitment.agent == agent);
        assert(commitment.releaseBps == releaseBps);
        assert(commitment.reputationBurned == expectedBurn);
        assert(commitment.cycleNumber == kenosis.currentCycle());
        assert(commitment.executed == false);
        assert(commitment.irrevocable == true);
    }

    // =========================================================================
    // Invariant 3: Burn Permanence
    // =========================================================================

    /**
     * @notice Verify that executed commitments permanently reduce token supply.
     * @dev Tests that burned tokens are actually destroyed.
     */
    function check_burn_permanence(bytes32 commitmentId, uint256 releaseBps) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);

        uint256 agentBalanceBefore = token.balanceOf(agent);
        uint256 expectedBurn = (agentBalanceBefore * releaseBps) / BPS_DENOMINATOR;
        vm.assume(expectedBurn > 0);

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        uint256 totalSupplyBefore = token.totalSupply();

        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        uint256 agentBalanceAfter = token.balanceOf(agent);
        uint256 totalSupplyAfter = token.totalSupply();

        // INVARIANT: Agent's balance reduced by burn amount
        assert(agentBalanceAfter == agentBalanceBefore - expectedBurn);

        // INVARIANT: Total supply reduced (tokens burned, not transferred)
        assert(totalSupplyAfter == totalSupplyBefore - expectedBurn);

        // INVARIANT: Total burned counter updated
        assert(kenosis.getTotalBurned() >= expectedBurn);
    }

    /**
     * @notice Verify commitment can only be executed once.
     * @dev Tests the executed flag prevents double execution.
     *
     * Single Execution Invariant:
     * Each commitment can only be executed exactly once.
     * This prevents double-burning and ensures burn accounting is accurate.
     */
    function check_single_execution(bytes32 commitmentId, uint256 releaseBps) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        // First execution should succeed
        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        KenosisBurn.KenosisCommitment memory commitment = kenosis.getCommitment(commitmentId);
        assert(commitment.executed == true);

        // Second execution should revert with custom error
        vm.prank(agent);
        vm.expectRevert(abi.encodeWithSelector(CommitmentAlreadyExecuted.selector, commitmentId));
        kenosis.executeKenosis(commitmentId);
    }

    // =========================================================================
    // Invariant 4: Commitment Uniqueness
    // =========================================================================

    /**
     * @notice Verify commitment IDs cannot be reused.
     * @dev Tests that duplicate commitments are rejected.
     *
     * Commitment Uniqueness Invariant:
     * Each commitment ID can only be used once.
     * This prevents replay attacks and ensures commitment tracking accuracy.
     */
    function check_commitment_uniqueness(bytes32 commitmentId, uint256 releaseBps) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS / 2); // Leave room for second

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        // Same ID should fail with custom error
        vm.prank(agent);
        vm.expectRevert(abi.encodeWithSelector(CommitmentAlreadyExists.selector, commitmentId));
        kenosis.commitKenosis(commitmentId, releaseBps);
    }

    // =========================================================================
    // Invariant 5: Cycle Boundary Behavior
    // =========================================================================

    /**
     * @notice Verify cycle advancement resets capacity for agent.
     * @dev Tests that capacity resets when cycle advances.
     */
    function check_cycle_reset_capacity(
        bytes32 commitmentId1,
        bytes32 commitmentId2,
        uint256 releaseBps
    ) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);
        vm.assume(commitmentId1 != commitmentId2);

        // Use full capacity in cycle 1
        vm.prank(agent);
        kenosis.commitKenosis(commitmentId1, releaseBps);

        uint256 capacityInCycle1 = kenosis.getRemainingCycleCapacity(agent);

        // Advance to cycle 2
        kenosis.advanceCycle();

        uint256 capacityInCycle2 = kenosis.getRemainingCycleCapacity(agent);

        // INVARIANT: Capacity resets in new cycle
        assert(capacityInCycle2 == MAX_RELEASE_BPS);

        // Can commit again in new cycle
        vm.prank(agent);
        kenosis.commitKenosis(commitmentId2, releaseBps);

        // Old cycle total unchanged
        assert(kenosis.getCycleReleaseTotal(agent, 1) == releaseBps);
        // New cycle has its own total
        assert(kenosis.getCycleReleaseTotal(agent, 2) == releaseBps);
    }

    // =========================================================================
    // Invariant 6: Only Owner Can Execute
    // =========================================================================

    /**
     * @notice Verify only the committing agent can execute their commitment.
     * @dev Tests access control on execution.
     *
     * Owner-Only Execution Invariant:
     * Only the agent who created the commitment can execute it.
     * This prevents unauthorized burning of another agent's reputation.
     */
    function check_execution_access_control(
        bytes32 commitmentId,
        uint256 releaseBps,
        address attacker
    ) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);
        vm.assume(attacker != agent);
        vm.assume(attacker != address(0));

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        // Attacker cannot execute with custom error
        vm.prank(attacker);
        vm.expectRevert(abi.encodeWithSelector(NotCommitmentOwner.selector, commitmentId, attacker, agent));
        kenosis.executeKenosis(commitmentId);

        // Owner can execute
        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        KenosisBurn.KenosisCommitment memory commitment = kenosis.getCommitment(commitmentId);
        assert(commitment.executed == true);
    }

    // =========================================================================
    // Invariant 7: Irrevocability Cannot Be Circumvented
    // =========================================================================

    /**
     * @notice Verify that no sequence of operations can undo irrevocability.
     * @dev Tests the permanence of the irrevocable state across all paths.
     */
    function check_irrevocability_across_cycles(
        bytes32 commitmentId,
        uint256 releaseBps
    ) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        // Irrevocable after commit
        assert(kenosis.isIrrevocable(commitmentId));

        // Advance multiple cycles
        kenosis.advanceCycle();
        kenosis.advanceCycle();
        kenosis.advanceCycle();

        // INVARIANT: Still irrevocable after cycle advances
        assert(kenosis.isIrrevocable(commitmentId));

        // Execute the commitment
        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        // Advance more cycles
        kenosis.advanceCycle();

        // INVARIANT: Still irrevocable after execution and more cycles
        assert(kenosis.isIrrevocable(commitmentId));
    }

    // =========================================================================
    // Invariant 8: Burn Amount Accuracy
    // =========================================================================

    /**
     * @notice Verify burn amount is calculated correctly as percentage of balance.
     * @dev Tests that reputationBurned = (balance * releaseBps) / 10000
     */
    function check_burn_amount_accuracy(
        bytes32 commitmentId,
        uint256 releaseBps,
        uint256 agentBalance
    ) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);
        vm.assume(agentBalance > 0 && agentBalance <= type(uint128).max);

        // Reset agent's balance
        uint256 currentBalance = token.balanceOf(agent);
        if (currentBalance > 0) {
            vm.prank(agent);
            token.transfer(address(1), currentBalance);
        }
        token.mint(agent, agentBalance);

        uint256 expectedBurn = (agentBalance * releaseBps) / BPS_DENOMINATOR;
        vm.assume(expectedBurn > 0); // Ensure non-zero burn

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        KenosisBurn.KenosisCommitment memory commitment = kenosis.getCommitment(commitmentId);

        // INVARIANT: Burn amount matches formula exactly
        assert(commitment.reputationBurned == expectedBurn);
    }
}
