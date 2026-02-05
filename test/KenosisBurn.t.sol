// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../contracts/KenosisBurn.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @dev Mock burnable ERC20 for testing.
contract MockReputationToken is ERC20 {
    constructor() ERC20("ReputationToken", "REP") {
        _mint(msg.sender, 1_000_000 ether);
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }

    function burn(uint256 amount) external {
        _burn(msg.sender, amount);
    }
}

contract KenosisBurnTest is Test {
    KenosisBurn public kenosis;
    MockReputationToken public token;

    address public admin = address(this);
    address public agent = address(0xBEEF);

    function setUp() public {
        token = new MockReputationToken();
        kenosis = new KenosisBurn(address(token));

        // Fund agent with reputation
        token.mint(agent, 1000 ether);
        vm.prank(agent);
        token.approve(address(kenosis), type(uint256).max);
    }

    // =========================================================================
    // Commitment
    // =========================================================================

    function test_commitKenosis_basic() public {
        bytes32 commitmentId = keccak256("commitment-1");
        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, 1000); // 10%

        (,,uint256 releaseBps,,,,, bool irrevocable) = kenosis.commitments(commitmentId);
        assertEq(releaseBps, 1000);
        assertTrue(irrevocable);
    }

    function test_commitKenosis_irrevocable() public {
        bytes32 commitmentId = keccak256("commitment-2");
        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, 500);

        assertTrue(kenosis.isIrrevocable(commitmentId));
    }

    // =========================================================================
    // 20% Cap Enforcement
    // =========================================================================

    function test_commitKenosis_enforces20PercentCap() public {
        bytes32 commitmentId = keccak256("commitment-3");

        // 20% = 2000 bps is the max
        vm.prank(agent);
        vm.expectRevert("KenosisBurn: exceeds 20% cap per cycle");
        kenosis.commitKenosis(commitmentId, 2001);
    }

    function test_commitKenosis_cycleCap_acrossMultiple() public {
        vm.startPrank(agent);

        // First commitment: 15%
        kenosis.commitKenosis(keccak256("c1"), 1500);

        // Second commitment: 6% would exceed 20% total
        vm.expectRevert("KenosisBurn: cycle cap exceeded");
        kenosis.commitKenosis(keccak256("c2"), 600);

        vm.stopPrank();
    }

    function test_commitKenosis_capacityResetsOnCycleAdvance() public {
        vm.prank(agent);
        kenosis.commitKenosis(keccak256("c3"), 2000); // max out cycle 1

        // Advance cycle
        kenosis.advanceCycle();

        // Should be able to commit again
        vm.prank(agent);
        kenosis.commitKenosis(keccak256("c4"), 2000); // max out cycle 2

        assertEq(kenosis.currentCycle(), 2);
    }

    // =========================================================================
    // Execution (Burn)
    // =========================================================================

    function test_executeKenosis_burnsPermanently() public {
        bytes32 commitmentId = keccak256("commitment-4");

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, 1000); // 10%

        uint256 balanceBefore = token.balanceOf(agent);

        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        uint256 balanceAfter = token.balanceOf(agent);
        assertTrue(balanceAfter < balanceBefore);
    }

    function test_executeKenosis_cannotExecuteTwice() public {
        bytes32 commitmentId = keccak256("commitment-5");

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, 500);

        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        vm.prank(agent);
        vm.expectRevert("KenosisBurn: already executed");
        kenosis.executeKenosis(commitmentId);
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    function test_getRemainingCycleCapacity() public {
        assertEq(kenosis.getRemainingCycleCapacity(agent), 2000);

        vm.prank(agent);
        kenosis.commitKenosis(keccak256("c5"), 800);

        assertEq(kenosis.getRemainingCycleCapacity(agent), 1200);
    }

    // =========================================================================
    // Fuzz Tests
    // =========================================================================

    /**
     * @notice Fuzz test: commitKenosis respects the 20% per-cycle cap (2000 bps)
     * @dev KEY INVARIANT: Maximum 20% reputation release per cycle
     * @param amount Basis points to attempt to commit
     */
    function testFuzz_commitKenosis_respectsCycleCap(uint256 amount) public {
        // Test the full range of bps values
        vm.assume(amount > 0 && amount <= 10000); // 0-100%

        bytes32 commitmentId = keccak256(abi.encodePacked("fuzz-cap", amount));

        vm.startPrank(agent);

        if (amount > 2000) {
            // Amounts exceeding 20% (2000 bps) should revert
            vm.expectRevert("KenosisBurn: exceeds 20% cap per cycle");
            kenosis.commitKenosis(commitmentId, amount);
        } else {
            // Valid amounts (<= 20%) should succeed
            kenosis.commitKenosis(commitmentId, amount);

            (,,uint256 releaseBps,,,,, bool irrevocable) = kenosis.commitments(commitmentId);
            assertEq(releaseBps, amount);
            assertTrue(irrevocable);

            // Verify cycle tracking
            uint256 cycleUsed = kenosis.getCycleReleaseTotal(agent, kenosis.currentCycle());
            assertEq(cycleUsed, amount);
        }

        vm.stopPrank();
    }

    /**
     * @notice Fuzz test: multiple commitments enforce cumulative 20% cap
     * @dev Tests that sum of all commitments in a cycle cannot exceed 2000 bps
     * @param amounts Array of commitment amounts to test
     */
    function testFuzz_multipleCommitments_cumulativeCap(uint256[] calldata amounts) public {
        // Limit array size for test tractability
        vm.assume(amounts.length > 0 && amounts.length <= 10);

        uint256 cumulativeBps = 0;

        vm.startPrank(agent);

        for (uint256 i = 0; i < amounts.length; i++) {
            // Bound each amount to valid range (1-2000 bps)
            uint256 amount = bound(amounts[i], 1, 2000);
            bytes32 commitmentId = keccak256(abi.encodePacked("fuzz-cumulative", i, amounts[i]));

            uint256 projectedTotal = cumulativeBps + amount;

            if (projectedTotal > 2000) {
                // This commitment would exceed cycle cap
                vm.expectRevert("KenosisBurn: cycle cap exceeded");
                kenosis.commitKenosis(commitmentId, amount);
                // Don't update cumulative since it failed
            } else {
                // This commitment is within bounds
                kenosis.commitKenosis(commitmentId, amount);
                cumulativeBps += amount;

                // Verify cumulative tracking
                uint256 cycleTotal = kenosis.getCycleReleaseTotal(agent, kenosis.currentCycle());
                assertEq(cycleTotal, cumulativeBps);

                // KEY INVARIANT: Cumulative can never exceed 2000 bps
                assertTrue(cycleTotal <= 2000, "Cumulative cycle releases must not exceed 20%");
            }

            // Stop if we've maxed out the cycle
            if (cumulativeBps >= 2000) {
                break;
            }
        }

        vm.stopPrank();

        // Final verification: remaining capacity should be consistent
        uint256 remaining = kenosis.getRemainingCycleCapacity(agent);
        assertEq(remaining, 2000 - cumulativeBps);
    }

    /**
     * @notice Fuzz test: executeKenosis permanently burns tokens
     * @dev KEY INVARIANT: Released reputation is permanently burned (not redistributed)
     * @param releaseBps Basis points to burn (1-2000)
     */
    function testFuzz_executeKenosis_burnsPermanently(uint256 releaseBps) public {
        // Bound to valid release range
        releaseBps = bound(releaseBps, 1, 2000);

        bytes32 commitmentId = keccak256(abi.encodePacked("fuzz-burn", releaseBps));

        // Record initial states
        uint256 agentBalanceBefore = token.balanceOf(agent);
        uint256 totalBurnedBefore = kenosis.getTotalBurned();

        // Calculate expected burn
        uint256 expectedBurn = (agentBalanceBefore * releaseBps) / 10000;
        vm.assume(expectedBurn > 0); // Ensure meaningful burn

        // Commit kenosis
        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, releaseBps);

        // Verify commitment is irrevocable
        assertTrue(kenosis.isIrrevocable(commitmentId), "Commitment must be irrevocable");

        // Execute the burn
        vm.prank(agent);
        kenosis.executeKenosis(commitmentId);

        // Verify agent's balance decreased
        uint256 agentBalanceAfter = token.balanceOf(agent);
        assertEq(
            agentBalanceAfter,
            agentBalanceBefore - expectedBurn,
            "Agent balance should decrease by burned amount"
        );

        // Verify total burned increased
        uint256 totalBurnedAfter = kenosis.getTotalBurned();
        assertEq(
            totalBurnedAfter,
            totalBurnedBefore + expectedBurn,
            "Total burned should increase by burned amount"
        );

        // Verify commitment is marked as executed
        (,,,,,, bool executed,) = kenosis.commitments(commitmentId);
        assertTrue(executed, "Commitment must be marked as executed");

        // KEY INVARIANT: Cannot execute twice (irrevocable and permanent)
        vm.prank(agent);
        vm.expectRevert("KenosisBurn: already executed");
        kenosis.executeKenosis(commitmentId);
    }

    /**
     * @notice Fuzz test: cycle capacity resets properly on advance
     * @dev Tests that capacity is restored after cycle advances
     * @param commitsBeforeAdvance Number of commits before advancing cycle
     * @param commitsAfterAdvance Number of commits after advancing cycle
     */
    function testFuzz_cycleAdvance_resetsCapacity(
        uint8 commitsBeforeAdvance,
        uint8 commitsAfterAdvance
    ) public {
        // Bound commits to reasonable numbers
        vm.assume(commitsBeforeAdvance >= 1 && commitsBeforeAdvance <= 5);
        vm.assume(commitsAfterAdvance >= 1 && commitsAfterAdvance <= 5);

        uint256 bpsPerCommit = 400; // 4% each, max 5 per cycle = 20%

        // Make commits in cycle 1
        vm.startPrank(agent);
        for (uint8 i = 0; i < commitsBeforeAdvance; i++) {
            bytes32 id = keccak256(abi.encodePacked("fuzz-cycle1", i));
            kenosis.commitKenosis(id, bpsPerCommit);
        }
        vm.stopPrank();

        uint256 cycle1Used = kenosis.getCycleReleaseTotal(agent, 1);
        assertEq(cycle1Used, commitsBeforeAdvance * bpsPerCommit);

        // Advance cycle
        kenosis.advanceCycle();
        assertEq(kenosis.currentCycle(), 2);

        // Capacity should be fully restored for new cycle
        uint256 newCapacity = kenosis.getRemainingCycleCapacity(agent);
        assertEq(newCapacity, 2000, "Capacity must reset to 20% after cycle advance");

        // Make commits in cycle 2
        vm.startPrank(agent);
        for (uint8 i = 0; i < commitsAfterAdvance; i++) {
            bytes32 id = keccak256(abi.encodePacked("fuzz-cycle2", i));
            kenosis.commitKenosis(id, bpsPerCommit);
        }
        vm.stopPrank();

        // Verify cycle 2 tracking is independent
        uint256 cycle2Used = kenosis.getCycleReleaseTotal(agent, 2);
        assertEq(cycle2Used, commitsAfterAdvance * bpsPerCommit);

        // Cycle 1 usage should be unchanged
        assertEq(kenosis.getCycleReleaseTotal(agent, 1), cycle1Used);
    }

    /**
     * @notice Fuzz test: zero balance agents cannot commit kenosis
     * @param releaseBps Any valid release amount
     */
    function testFuzz_commitKenosis_requiresBalance(uint256 releaseBps) public {
        releaseBps = bound(releaseBps, 1, 2000);

        address poorAgent = address(0xDEAD);
        vm.prank(poorAgent);
        token.approve(address(kenosis), type(uint256).max);

        // Agent with zero balance should not be able to commit
        assertEq(token.balanceOf(poorAgent), 0);

        bytes32 commitmentId = keccak256(abi.encodePacked("fuzz-poor", releaseBps));

        vm.prank(poorAgent);
        vm.expectRevert("KenosisBurn: no reputation to release");
        kenosis.commitKenosis(commitmentId, releaseBps);
    }
}
