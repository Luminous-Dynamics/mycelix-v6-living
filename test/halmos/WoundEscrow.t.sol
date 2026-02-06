// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../../contracts/WoundEscrow.sol";
import "../../contracts/libraries/Errors.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/**
 * @title WoundEscrow Formal Verification Tests (Halmos)
 * @notice Symbolic execution tests for critical WoundEscrow invariants.
 * @dev Run with: halmos --contract WoundEscrowHalmosTest --solver-timeout-assertion 300
 *
 * Critical Invariants Verified:
 *   1. Phase transitions are forward-only (Gate 1)
 *   2. Escrow balance conservation: contract balance >= sum of all escrowed amounts
 *   3. Wound state consistency after operations
 *
 * Halmos Symbolic Execution Patterns:
 *   - Functions prefixed with `check_` are symbolic tests
 *   - vm.assume() constrains symbolic inputs
 *   - assert() statements define invariants to verify
 */

/// @dev Mock ERC20 for symbolic testing.
contract SymbolicFlowToken is ERC20 {
    constructor() ERC20("FlowToken", "FLOW") {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract WoundEscrowHalmosTest is Test {
    WoundEscrow public escrow;
    SymbolicFlowToken public token;

    address public admin;
    address public agent;
    address public healer;

    function setUp() public {
        admin = address(this);
        agent = address(0xBEEF);
        healer = address(0xCAFE);

        token = new SymbolicFlowToken();
        escrow = new WoundEscrow(address(token));

        escrow.grantRole(escrow.HEALER_ROLE(), healer);
        escrow.grantRole(escrow.VALIDATOR_ROLE(), admin);

        // Fund agent with large balance for symbolic testing
        token.mint(agent, type(uint128).max);
        vm.prank(agent);
        token.approve(address(escrow), type(uint256).max);
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    /// @dev Helper to get wound data from the contract's tuple return.
    function _getWoundEscrowAmount(bytes32 woundId) internal view returns (uint256) {
        (,,,, uint256 escrowAmount,,,,, bool exists) = escrow.getWound(woundId);
        require(exists, "Wound does not exist");
        return escrowAmount;
    }

    /// @dev Helper to check if wound exists.
    function _woundExists(bytes32 woundId) internal view returns (bool) {
        try escrow.getWoundPhase(woundId) returns (WoundEscrow.WoundPhase) {
            return true;
        } catch {
            return false;
        }
    }

    // =========================================================================
    // Invariant 1: Phase Forward-Only (Gate 1)
    // =========================================================================

    /**
     * @notice Verify that advancePhase always moves phase forward by exactly 1.
     * @dev Symbolically tests that no code path exists where phase decreases or skips.
     *
     * This is the critical Gate 1 invariant from the Constitution:
     * "Wound phases advance forward only" - once a wound progresses through
     * Hemostasis -> Inflammation -> Proliferation -> Remodeling -> Healed,
     * it can never go backwards.
     */
    function check_phase_forward_only(bytes32 woundId, uint256 escrowAmount) public {
        // Bound escrow amount to valid range
        vm.assume(escrowAmount > 0 && escrowAmount <= type(uint96).max);

        // Create wound in Hemostasis
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            escrowAmount,
            0 // No restitution required for simple test
        );

        // Get phase before advance
        uint8 phaseBefore = uint8(escrow.getWoundPhase(woundId));

        // Fast-forward time past hemostasis requirement
        vm.warp(block.timestamp + 2 hours);

        // Only advance if not already healed
        if (phaseBefore < uint8(WoundEscrow.WoundPhase.Healed)) {
            vm.prank(healer);
            escrow.advancePhase(woundId);

            uint8 phaseAfter = uint8(escrow.getWoundPhase(woundId));

            // CRITICAL INVARIANT: Phase must advance by exactly 1
            assert(phaseAfter == phaseBefore + 1);
            // Phase can never decrease
            assert(phaseAfter > phaseBefore);
        }
    }

    /**
     * @notice Verify that phase never decreases across multiple advances.
     * @dev Tests the monotonicity property of phase transitions.
     */
    function check_phase_monotonic(bytes32 woundId, uint8 advanceCount) public {
        vm.assume(advanceCount > 0 && advanceCount <= 10);

        // Create wound
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            1 ether,
            0
        );

        vm.warp(block.timestamp + 2 hours);

        uint8 previousPhase = 0; // Hemostasis

        for (uint8 i = 0; i < advanceCount; i++) {
            uint8 currentPhase = uint8(escrow.getWoundPhase(woundId));

            // INVARIANT: Phase is always >= previous phase
            assert(currentPhase >= previousPhase);

            if (currentPhase < uint8(WoundEscrow.WoundPhase.Healed)) {
                vm.prank(healer);
                escrow.advancePhase(woundId);
                previousPhase = currentPhase;
            } else {
                // At Healed, should stay at Healed
                assert(currentPhase == uint8(WoundEscrow.WoundPhase.Healed));
                break;
            }
        }
    }

    // =========================================================================
    // Invariant 2: Escrow Balance Conservation
    // =========================================================================

    /**
     * @notice Verify that contract balance is always >= total escrowed amount.
     * @dev Ensures no funds can be extracted beyond what's properly released.
     *
     * Escrow Conservation Invariant:
     * For all active wounds: sum(wound.escrowAmount) <= token.balanceOf(contract)
     * This ensures the contract always has sufficient funds to release escrow
     * when wounds are healed.
     */
    function check_escrow_conservation_on_create(
        bytes32 woundId,
        uint256 escrowAmount
    ) public {
        vm.assume(escrowAmount > 0 && escrowAmount <= type(uint96).max);

        uint256 contractBalanceBefore = token.balanceOf(address(escrow));

        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            escrowAmount,
            0
        );

        uint256 contractBalanceAfter = token.balanceOf(address(escrow));
        uint256 recordedEscrow = _getWoundEscrowAmount(woundId);

        // INVARIANT: Contract received exactly the escrow amount
        assert(contractBalanceAfter == contractBalanceBefore + escrowAmount);

        // INVARIANT: Wound records correct escrow amount
        assert(recordedEscrow == escrowAmount);

        // INVARIANT: Contract balance >= recorded escrow (conservation)
        assert(contractBalanceAfter >= recordedEscrow);
    }

    /**
     * @notice Verify escrow is properly released only when wound is healed.
     * @dev Tests the complete healing cycle preserves balance invariant.
     *
     * This test verifies that:
     * 1. Escrow is held by the contract during all phases before Healed
     * 2. Escrow is released to the agent only upon reaching Healed phase
     * 3. The wound's escrow amount is zeroed after release
     */
    function check_escrow_released_only_on_heal(
        bytes32 woundId,
        uint256 escrowAmount
    ) public {
        vm.assume(escrowAmount > 0 && escrowAmount <= type(uint96).max);

        // Create wound
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            escrowAmount,
            0
        );

        uint256 agentBalanceBefore = token.balanceOf(agent);
        uint256 contractBalance = token.balanceOf(address(escrow));

        // Contract should hold the escrow
        assert(contractBalance >= escrowAmount);

        // Advance through all phases
        vm.warp(block.timestamp + 2 hours);
        vm.startPrank(healer);
        escrow.advancePhase(woundId); // -> Inflammation
        escrow.advancePhase(woundId); // -> Proliferation
        escrow.advancePhase(woundId); // -> Remodeling

        // Before final advance, escrow still held
        assert(token.balanceOf(address(escrow)) >= escrowAmount);

        escrow.advancePhase(woundId); // -> Healed (releases escrow)
        vm.stopPrank();

        // INVARIANT: After healing, agent receives escrow back
        uint256 agentBalanceAfter = token.balanceOf(agent);
        assert(agentBalanceAfter == agentBalanceBefore + escrowAmount);

        // INVARIANT: Wound's escrow amount is now 0
        uint256 recordedEscrow = _getWoundEscrowAmount(woundId);
        assert(recordedEscrow == 0);
    }

    // =========================================================================
    // Invariant 3: Restitution Bounds
    // =========================================================================

    /**
     * @notice Verify restitution paid never exceeds restitution required.
     * @dev Tests the payment capping logic.
     *
     * Restitution Bounds Invariant:
     * For any wound: restitutionPaid <= restitutionRequired
     * Excess payments are rejected to prevent overpayment.
     */
    function check_restitution_capped(
        bytes32 woundId,
        uint256 escrowAmount,
        uint256 restitutionRequired,
        uint256 paymentAmount
    ) public {
        vm.assume(escrowAmount > 0 && escrowAmount <= type(uint96).max);
        vm.assume(restitutionRequired > 0 && restitutionRequired <= escrowAmount);
        vm.assume(paymentAmount > 0 && paymentAmount <= type(uint96).max);

        // Create wound with restitution requirement
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Moderate,
            escrowAmount,
            restitutionRequired
        );

        // Advance to Proliferation where restitution can be paid
        vm.warp(block.timestamp + 2 hours);
        vm.startPrank(healer);
        escrow.advancePhase(woundId); // -> Inflammation
        escrow.advancePhase(woundId); // -> Proliferation
        vm.stopPrank();

        // Pay restitution
        token.mint(agent, paymentAmount);
        vm.prank(agent);
        escrow.payRestitution(woundId, paymentAmount);

        // Get the restitution amounts from the tuple return
        (,,,,,uint256 restitutionReq, uint256 restitutionPaid,,,) = escrow.getWound(woundId);

        // INVARIANT: Restitution paid is capped at required amount
        assert(restitutionPaid <= restitutionReq);
    }

    // =========================================================================
    // Invariant 4: Wound Existence and Uniqueness
    // =========================================================================

    /**
     * @notice Verify wound IDs are unique and cannot be reused.
     * @dev Tests that creating a duplicate wound reverts.
     *
     * Uniqueness Invariant:
     * Each wound ID can only be used once. Attempting to create a wound
     * with an existing ID will revert with WoundAlreadyExists.
     */
    function check_wound_uniqueness(bytes32 woundId, uint256 escrowAmount) public {
        vm.assume(escrowAmount > 0 && escrowAmount <= type(uint96).max);

        // Create first wound
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            escrowAmount,
            0
        );

        // Wound should now exist
        assert(_woundExists(woundId) == true);

        // Second creation with same ID should revert with custom error
        vm.expectRevert(abi.encodeWithSelector(WoundAlreadyExists.selector, woundId));
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            escrowAmount,
            0
        );
    }

    // =========================================================================
    // Invariant 5: State Consistency
    // =========================================================================

    /**
     * @notice Verify wound state is consistent after creation.
     * @dev Tests all wound fields are properly initialized.
     *
     * State Consistency Invariant:
     * After creation, all wound fields must be properly initialized:
     * - Phase starts at Hemostasis
     * - Timestamps are set to current block
     * - Restitution paid starts at 0
     */
    function check_wound_creation_consistency(
        bytes32 woundId,
        uint8 severityVal,
        uint256 escrowAmount,
        uint256 restitutionRequired
    ) public {
        vm.assume(severityVal <= 3); // Valid severity range
        vm.assume(escrowAmount > 0 && escrowAmount <= type(uint96).max);
        vm.assume(restitutionRequired <= type(uint96).max);
        vm.assume(restitutionRequired <= escrowAmount);

        WoundEscrow.WoundSeverity severity = WoundEscrow.WoundSeverity(severityVal);

        escrow.createWound(
            woundId,
            agent,
            severity,
            escrowAmount,
            restitutionRequired
        );

        // Get wound data from tuple return
        (
            bytes32 _woundId,
            address woundAgent,
            WoundEscrow.WoundSeverity woundSeverity,
            WoundEscrow.WoundPhase woundPhase,
            uint256 woundEscrowAmount,
            uint256 woundRestitutionRequired,
            uint256 woundRestitutionPaid,
            uint256 createdAt,
            uint256 lastPhaseChange,
            bool exists
        ) = escrow.getWound(woundId);

        // INVARIANTS: All fields properly set
        assert(_woundId == woundId);
        assert(woundAgent == agent);
        assert(woundSeverity == severity);
        assert(woundPhase == WoundEscrow.WoundPhase.Hemostasis);
        assert(woundEscrowAmount == escrowAmount);
        assert(woundRestitutionRequired == restitutionRequired);
        assert(woundRestitutionPaid == 0);
        assert(createdAt == block.timestamp);
        assert(lastPhaseChange == block.timestamp);
        assert(exists == true);
    }

    // =========================================================================
    // Invariant 6: Phase Cannot Skip
    // =========================================================================

    /**
     * @notice Verify that phases cannot be skipped during healing.
     * @dev Each phase transition must go through all intermediate phases.
     */
    function check_phase_cannot_skip(bytes32 woundId, uint256 escrowAmount) public {
        vm.assume(escrowAmount > 0 && escrowAmount <= type(uint96).max);

        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            escrowAmount,
            0
        );

        // Phase should start at Hemostasis (0)
        assert(uint8(escrow.getWoundPhase(woundId)) == 0);

        vm.warp(block.timestamp + 2 hours);
        vm.prank(healer);
        escrow.advancePhase(woundId);

        // Must be at Inflammation (1), not any later phase
        assert(uint8(escrow.getWoundPhase(woundId)) == 1);

        vm.prank(healer);
        escrow.advancePhase(woundId);

        // Must be at Proliferation (2)
        assert(uint8(escrow.getWoundPhase(woundId)) == 2);
    }

    // =========================================================================
    // Invariant 7: Multiple Wounds Conservation
    // =========================================================================

    /**
     * @notice Verify conservation across multiple wounds.
     * @dev Contract balance must be >= sum of all active wound escrows.
     */
    function check_multi_wound_conservation(
        bytes32 woundId1,
        bytes32 woundId2,
        uint256 escrow1,
        uint256 escrow2
    ) public {
        vm.assume(woundId1 != woundId2);
        vm.assume(escrow1 > 0 && escrow1 <= type(uint64).max);
        vm.assume(escrow2 > 0 && escrow2 <= type(uint64).max);

        // Create two wounds
        escrow.createWound(woundId1, agent, WoundEscrow.WoundSeverity.Minor, escrow1, 0);
        escrow.createWound(woundId2, agent, WoundEscrow.WoundSeverity.Moderate, escrow2, 0);

        uint256 totalEscrowed = escrow1 + escrow2;
        uint256 contractBalance = token.balanceOf(address(escrow));

        // INVARIANT: Contract balance >= total escrowed
        assert(contractBalance >= totalEscrowed);
    }
}
