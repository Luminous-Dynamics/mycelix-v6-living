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
}
