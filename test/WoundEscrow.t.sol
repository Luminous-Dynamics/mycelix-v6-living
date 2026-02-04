// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../contracts/WoundEscrow.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @dev Mock ERC20 for testing.
contract MockFlowToken is ERC20 {
    constructor() ERC20("FlowToken", "FLOW") {
        _mint(msg.sender, 1_000_000 ether);
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract WoundEscrowTest is Test {
    WoundEscrow public escrow;
    MockFlowToken public token;

    address public admin = address(this);
    address public agent = address(0xBEEF);
    address public healer = address(0xCAFE);

    function setUp() public {
        token = new MockFlowToken();
        escrow = new WoundEscrow(address(token));

        // Grant healer role
        escrow.grantRole(escrow.HEALER_ROLE(), healer);

        // Fund agent
        token.mint(agent, 100 ether);
        vm.prank(agent);
        token.approve(address(escrow), type(uint256).max);
    }

    // =========================================================================
    // Wound Creation
    // =========================================================================

    function test_createWound_startsInHemostasis() public {
        bytes32 woundId = keccak256("wound-1");
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Moderate,
            10 ether,
            5 ether
        );

        assertEq(
            uint256(escrow.getWoundPhase(woundId)),
            uint256(WoundEscrow.WoundPhase.Hemostasis)
        );
    }

    function test_createWound_transfersEscrow() public {
        bytes32 woundId = keccak256("wound-2");
        uint256 balanceBefore = token.balanceOf(agent);

        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            5 ether,
            2 ether
        );

        assertEq(token.balanceOf(agent), balanceBefore - 5 ether);
        assertEq(token.balanceOf(address(escrow)), 5 ether);
    }

    function test_createWound_revertsDuplicate() public {
        bytes32 woundId = keccak256("wound-3");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 1 ether);

        vm.expectRevert("WoundEscrow: wound already exists");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 1 ether);
    }

    // =========================================================================
    // Phase Advancement (forward-only invariant)
    // =========================================================================

    function test_advancePhase_forwardOnly() public {
        bytes32 woundId = keccak256("wound-4");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 0);

        // Hemostasis -> Inflammation (requires min duration)
        vm.warp(block.timestamp + 2 hours);
        vm.prank(healer);
        escrow.advancePhase(woundId);
        assertEq(
            uint256(escrow.getWoundPhase(woundId)),
            uint256(WoundEscrow.WoundPhase.Inflammation)
        );

        // Inflammation -> Proliferation
        vm.prank(healer);
        escrow.advancePhase(woundId);
        assertEq(
            uint256(escrow.getWoundPhase(woundId)),
            uint256(WoundEscrow.WoundPhase.Proliferation)
        );
    }

    function test_advancePhase_requiresMinHemostasis() public {
        bytes32 woundId = keccak256("wound-5");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 0);

        // Try to advance before min hemostasis duration
        vm.prank(healer);
        vm.expectRevert("WoundEscrow: minimum hemostasis not elapsed");
        escrow.advancePhase(woundId);
    }

    function test_advancePhase_cannotAdvancePastHealed() public {
        bytes32 woundId = keccak256("wound-6");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 0);

        // Advance through all phases
        vm.warp(block.timestamp + 2 hours);
        vm.startPrank(healer);
        escrow.advancePhase(woundId); // -> Inflammation
        escrow.advancePhase(woundId); // -> Proliferation
        escrow.advancePhase(woundId); // -> Remodeling
        escrow.advancePhase(woundId); // -> Healed
        vm.stopPrank();

        // Cannot advance past Healed
        vm.prank(healer);
        vm.expectRevert("WoundEscrow: already healed");
        escrow.advancePhase(woundId);
    }

    // =========================================================================
    // Restitution
    // =========================================================================

    function test_payRestitution_requiresProliferationPhase() public {
        bytes32 woundId = keccak256("wound-7");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 1 ether);

        // Cannot pay restitution in hemostasis
        vm.prank(agent);
        vm.expectRevert("WoundEscrow: not in proliferation phase");
        escrow.payRestitution(woundId, 0.5 ether);
    }

    function test_fullHealingCycle_releasesEscrow() public {
        bytes32 woundId = keccak256("wound-8");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 5 ether, 2 ether);

        vm.warp(block.timestamp + 2 hours);
        vm.startPrank(healer);
        escrow.advancePhase(woundId); // -> Inflammation
        escrow.advancePhase(woundId); // -> Proliferation
        vm.stopPrank();

        // Pay restitution
        token.mint(agent, 2 ether);
        vm.prank(agent);
        escrow.payRestitution(woundId, 2 ether);

        // Advance to healed
        vm.startPrank(healer);
        escrow.advancePhase(woundId); // -> Remodeling
        escrow.advancePhase(woundId); // -> Healed (releases escrow)
        vm.stopPrank();

        assertTrue(escrow.isHealed(woundId));
    }
}
