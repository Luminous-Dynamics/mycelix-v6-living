// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../../contracts/KenosisBurn.sol";
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
     */
    function check_max_release_single_commitment(
        bytes32 commitmentId,
        uint256 releaseBps
    ) public {
        vm.assume(releaseBps > 0);

        vm.prank(agent);

        if (releaseBps > MAX_RELEASE_BPS) {
            // Should revert for > 20%
            vm.expectRevert("KenosisBurn: exceeds 20% cap per cycle");
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

        // First commitment
        vm.prank(agent);
        kenosis.commitKenosis(commitmentId1, release1Bps);

        uint256 cycleTotal = kenosis.getCycleReleaseTotal(agent, kenosis.currentCycle());
        assert(cycleTotal == release1Bps);

        // Second commitment
        vm.prank(agent);

        if (release1Bps + release2Bps > MAX_RELEASE_BPS) {
            // Should revert if cumulative > 20%
            vm.expectRevert("KenosisBurn: cycle cap exceeded");
            kenosis.commitKenosis(commitmentId2, release2Bps);
        } else {
            // Should succeed if cumulative <= 20%
            kenosis.commitKenosis(commitmentId2, release2Bps);

            uint256 newTotal = kenosis.getCycleReleaseTotal(agent, kenosis.currentCycle());
            assert(newTotal == release1Bps + release2Bps);
            assert(newTotal <= MAX_RELEASE_BPS);
        }
    }

    /**
     * @notice Verify remaining capacity calculation is correct.
     * @dev Tests the getRemainingCycleCapacity function.
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
     */
    function check_irrevocable_permanent(bytes32 commitmentId, uint256 releaseBps) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS);

        // Before commitment, should not exist
        assert(!kenosis.isIrrevocable(commitmentId));

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        // INVARIANT: Once committed, irrevocable is permanently true
        assert(kenosis.isIrrevocable(commitmentId));

        // Even after execution, still irrevocable
        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

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

        // Second execution should revert
        vm.prank(agent);
        vm.expectRevert("KenosisBurn: already executed");
        kenosis.executeKenosis(commitmentId);
    }

    // =========================================================================
    // Invariant 4: Commitment Uniqueness
    // =========================================================================

    /**
     * @notice Verify commitment IDs cannot be reused.
     * @dev Tests that duplicate commitments are rejected.
     */
    function check_commitment_uniqueness(bytes32 commitmentId, uint256 releaseBps) public {
        vm.assume(releaseBps > 0 && releaseBps <= MAX_RELEASE_BPS / 2); // Leave room for second

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        // Same ID should fail
        vm.prank(agent);
        vm.expectRevert("KenosisBurn: commitment exists");
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

        // Attacker cannot execute
        vm.prank(attacker);
        vm.expectRevert("KenosisBurn: not your commitment");
        kenosis.executeKenosis(commitmentId);

        // Owner can execute
        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        KenosisBurn.KenosisCommitment memory commitment = kenosis.getCommitment(commitmentId);
        assert(commitment.executed == true);
    }
}
