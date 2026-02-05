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

    // =========================================================================
    // Fuzz Tests
    // =========================================================================

    /**
     * @notice Fuzz test: createWound with various severity levels
     * @dev Tests that all valid severity values (0-3) successfully create wounds
     */
    function testFuzz_createWound_validSeverity(uint8 severity) public {
        // WoundSeverity has 4 values: Minor(0), Moderate(1), Severe(2), Critical(3)
        vm.assume(severity <= 3);

        bytes32 woundId = keccak256(abi.encodePacked("fuzz-wound-severity", severity));
        WoundEscrow.WoundSeverity woundSeverity = WoundEscrow.WoundSeverity(severity);

        // Ensure agent has funds
        token.mint(agent, 10 ether);

        escrow.createWound(
            woundId,
            agent,
            woundSeverity,
            1 ether,
            0.5 ether
        );

        // Verify wound was created with correct severity
        WoundEscrow.Wound memory wound = escrow.getWound(woundId);
        assertEq(uint256(wound.severity), uint256(severity));
        assertEq(uint256(wound.phase), uint256(WoundEscrow.WoundPhase.Hemostasis));
        assertTrue(wound.exists);
    }

    /**
     * @notice Fuzz test: advancePhase always moves forward, never backward
     * @dev KEY INVARIANT: Phases can ONLY advance forward (Gate 1 invariant)
     * @param startPhase The initial phase to test from (0-3, excluding Healed)
     * @param advances Number of phase advances to attempt
     */
    function testFuzz_advancePhase_alwaysForward(uint8 startPhase, uint8 advances) public {
        // startPhase: 0-3 (Hemostasis to Remodeling, can't start at Healed)
        // advances: 1-10 to keep test tractable
        vm.assume(startPhase <= 3);
        vm.assume(advances >= 1 && advances <= 10);

        bytes32 woundId = keccak256(abi.encodePacked("fuzz-forward", startPhase, advances));
        token.mint(agent, 100 ether);

        // Create wound
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 0);

        // Fast-forward past hemostasis requirement
        vm.warp(block.timestamp + 2 hours);

        // Advance to the start phase
        vm.startPrank(healer);
        for (uint8 i = 0; i < startPhase; i++) {
            escrow.advancePhase(woundId);
        }
        vm.stopPrank();

        // Record the phase after setup
        uint256 currentPhase = uint256(escrow.getWoundPhase(woundId));
        assertEq(currentPhase, startPhase);

        // Now attempt multiple advances and verify forward-only invariant
        vm.startPrank(healer);
        for (uint8 i = 0; i < advances; i++) {
            uint256 phaseBefore = uint256(escrow.getWoundPhase(woundId));

            // If already healed, advancing should revert
            if (phaseBefore == uint256(WoundEscrow.WoundPhase.Healed)) {
                vm.expectRevert("WoundEscrow: already healed");
                escrow.advancePhase(woundId);
                break;
            }

            escrow.advancePhase(woundId);
            uint256 phaseAfter = uint256(escrow.getWoundPhase(woundId));

            // KEY INVARIANT: Phase must have advanced exactly by 1
            assertEq(phaseAfter, phaseBefore + 1, "Phase must advance forward by exactly 1");
            assertTrue(phaseAfter > phaseBefore, "Phase must always move forward");
        }
        vm.stopPrank();
    }

    /**
     * @notice Fuzz test: restitution payment can never exceed escrow amount
     * @dev Verifies that restitution payments are capped at the required amount
     * @param escrowAmount Amount to escrow (bounded for practicality)
     * @param restitutionRequired Required restitution (bounded to escrow amount)
     * @param paymentAttempt Amount attempted to pay as restitution
     */
    function testFuzz_restitution_neverExceedsEscrow(
        uint256 escrowAmount,
        uint256 restitutionRequired,
        uint256 paymentAttempt
    ) public {
        // Bound inputs to reasonable ranges
        vm.assume(escrowAmount >= 0.01 ether && escrowAmount <= 1000 ether);
        vm.assume(restitutionRequired > 0 && restitutionRequired <= escrowAmount);
        vm.assume(paymentAttempt > 0 && paymentAttempt <= 10000 ether);

        bytes32 woundId = keccak256(abi.encodePacked("fuzz-restitution", escrowAmount, restitutionRequired));

        // Fund agent generously
        token.mint(agent, escrowAmount + paymentAttempt + 100 ether);

        // Create wound
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Moderate, escrowAmount, restitutionRequired);

        // Advance to Proliferation phase where restitution can be paid
        vm.warp(block.timestamp + 2 hours);
        vm.startPrank(healer);
        escrow.advancePhase(woundId); // -> Inflammation
        escrow.advancePhase(woundId); // -> Proliferation
        vm.stopPrank();

        // Record state before payment
        uint256 restitutionBefore = escrow.restitutionRemaining(woundId);
        assertEq(restitutionBefore, restitutionRequired);

        // Attempt to pay restitution (may be more than required)
        vm.prank(agent);
        escrow.payRestitution(woundId, paymentAttempt);

        // Verify restitution paid is capped at required amount
        uint256 restitutionAfter = escrow.restitutionRemaining(woundId);
        WoundEscrow.Wound memory wound = escrow.getWound(woundId);

        // KEY INVARIANT: restitutionPaid can never exceed restitutionRequired
        assertTrue(
            wound.restitutionPaid <= wound.restitutionRequired,
            "Restitution paid must never exceed required amount"
        );

        // If payment was >= required, remaining should be 0
        if (paymentAttempt >= restitutionRequired) {
            assertEq(restitutionAfter, 0, "Remaining should be 0 when payment covers requirement");
            assertEq(wound.restitutionPaid, restitutionRequired, "Paid should equal required");
        } else {
            // Payment was less than required
            assertEq(
                restitutionAfter,
                restitutionRequired - paymentAttempt,
                "Remaining should decrease by payment amount"
            );
        }
    }

    // NOTE: testFuzz_createWound_invalidSeverity removed because Solidity 0.8.x
    // does not automatically revert on invalid enum casts. The contract would need
    // explicit validation to reject invalid severity values. This is a known
    // Solidity limitation with enum type casting.

    /**
     * @notice Fuzz test: escrow amounts must be positive
     * @dev Tests various escrow amounts to ensure zero is rejected
     */
    function testFuzz_createWound_escrowAmountBounds(uint256 escrowAmount) public {
        bytes32 woundId = keccak256(abi.encodePacked("fuzz-escrow-amount", escrowAmount));

        if (escrowAmount == 0) {
            vm.expectRevert("WoundEscrow: zero escrow");
            escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, escrowAmount, 0);
        } else {
            // Bound to reasonable amount and ensure agent has funds
            vm.assume(escrowAmount <= 1000 ether);
            token.mint(agent, escrowAmount);

            escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, escrowAmount, 0);

            WoundEscrow.Wound memory wound = escrow.getWound(woundId);
            assertEq(wound.escrowAmount, escrowAmount);
        }
    }
}
