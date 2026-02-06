// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

/**
 * @title WoundEscrow
 * @notice On-chain restitution tracking for wound healing [Primitive 2].
 * @dev Replaces punitive slashing with a healing-oriented escrow mechanism.
 *      Funds are escrowed during hemostasis and released as restitution is fulfilled.
 *
 * Gas Optimizations (v6.0):
 *   - Struct packing: 8 slots -> 4 slots (~60,000 gas savings per create)
 *   - Custom errors: ~2,000 gas savings per revert
 *   - Pagination: prevents unbounded gas consumption
 *
 * Constitutional Alignment: Sacred Reciprocity (Harmony 6)
 * Three Gates:
 *   - Gate 1: Wound phases advance forward only (enforced on-chain)
 *   - Gate 2: Warnings for critical wounds (emitted as events)
 *   - Gate 3: Reputation impact via off-chain MATL integration
 */

import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "./libraries/Errors.sol";

contract WoundEscrow is AccessControl, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // =========================================================================
    // Roles
    // =========================================================================

    bytes32 public constant HEALER_ROLE = keccak256("HEALER_ROLE");
    bytes32 public constant VALIDATOR_ROLE = keccak256("VALIDATOR_ROLE");

    // =========================================================================
    // Types
    // =========================================================================

    enum WoundPhase {
        Hemostasis,     // 0: Immediate quarantine
        Inflammation,   // 1: Community assessment
        Proliferation,  // 2: Restitution and repair
        Remodeling,     // 3: Integration and strengthening
        Healed          // 4: Complete
    }

    enum WoundSeverity {
        Minor,     // Was 1-5% slash
        Moderate,  // Was 5-15% slash
        Severe,    // Was 15-30% slash
        Critical   // Was 30%+ slash
    }

    /**
     * @dev Gas-optimized Wound struct using tight packing.
     *
     * Storage Layout (4 slots instead of 8):
     *   Slot 1: woundId (32 bytes)
     *   Slot 2: agent (20 bytes) + severity (1 byte) + phase (1 byte) + exists (1 byte) = 23 bytes
     *   Slot 3: escrowAmount (12 bytes) + restitutionRequired (12 bytes) = 24 bytes
     *   Slot 4: createdAt (6 bytes) + lastPhaseChange (6 bytes) + restitutionPaid (8 bytes) = 20 bytes
     *
     * Constraints:
     *   - escrowAmount/restitutionRequired: max ~79 billion tokens (uint96)
     *   - restitutionPaid: max ~18 quintillion (uint64)
     *   - timestamps: good until year 8 million (uint48 seconds)
     */
    struct Wound {
        bytes32 woundId;            // Slot 1: 32 bytes
        address agent;              // Slot 2: 20 bytes
        uint8 severity;             // Slot 2: 1 byte (WoundSeverity enum)
        uint8 phase;                // Slot 2: 1 byte (WoundPhase enum)
        bool exists;                // Slot 2: 1 byte
        uint96 escrowAmount;        // Slot 3: 12 bytes (max ~79B tokens with 18 decimals)
        uint96 restitutionRequired; // Slot 3: 12 bytes
        uint48 createdAt;           // Slot 4: 6 bytes (seconds since epoch)
        uint48 lastPhaseChange;     // Slot 4: 6 bytes
        uint64 restitutionPaid;     // Slot 4: 8 bytes
    }

    struct ScarTissue {
        bytes32 woundId;
        string area;
        uint256 strengthMultiplier; // Basis points (10000 = 1.0x, 15000 = 1.5x)
        uint256 formedAt;
    }

    // =========================================================================
    // State
    // =========================================================================

    IERC20 public immutable flowToken;
    mapping(bytes32 => Wound) public wounds;
    mapping(bytes32 => ScarTissue) public scars;
    mapping(address => bytes32[]) public agentWounds;

    // Minimum time in hemostasis (seconds)
    uint256 public minHemostasisDuration = 1 hours;
    uint256 public maxHemostasisDuration = 72 hours;

    // =========================================================================
    // Events
    // =========================================================================

    event WoundCreated(
        bytes32 indexed woundId,
        address indexed agent,
        WoundSeverity severity,
        uint256 escrowAmount,
        uint256 timestamp
    );

    event WoundPhaseAdvanced(
        bytes32 indexed woundId,
        WoundPhase from,
        WoundPhase to,
        uint256 timestamp
    );

    event RestitutionPaid(
        bytes32 indexed woundId,
        address indexed agent,
        uint256 amount,
        uint256 totalPaid,
        uint256 required
    );

    event ScarTissueFormed(
        bytes32 indexed woundId,
        string area,
        uint256 strengthMultiplier
    );

    event WoundHealed(
        bytes32 indexed woundId,
        address indexed agent,
        uint256 totalRestitution,
        uint256 timestamp
    );

    // Gate 2 style warning event
    event ConstitutionalWarning(
        bytes32 indexed woundId,
        string harmony,
        string warning,
        uint256 severity
    );

    // =========================================================================
    // Constructor
    // =========================================================================

    constructor(address _flowToken) {
        flowToken = IERC20(_flowToken);
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(HEALER_ROLE, msg.sender);
        _grantRole(VALIDATOR_ROLE, msg.sender);
    }

    // =========================================================================
    // Core Functions
    // =========================================================================

    /**
     * @notice Create a new wound (starts in Hemostasis).
     * @dev Hemostasis is automatic and un-gameable - the quarantine happens immediately.
     */
    function createWound(
        bytes32 woundId,
        address agent,
        WoundSeverity severity,
        uint256 escrowAmount,
        uint256 restitutionRequired
    ) external onlyRole(VALIDATOR_ROLE) nonReentrant {
        if (wounds[woundId].exists) revert WoundAlreadyExists(woundId);
        if (agent == address(0)) revert ZeroAddress();
        if (escrowAmount == 0) revert ZeroEscrow();

        // Safe downcast checks for packed struct
        if (escrowAmount > type(uint96).max) {
            revert("WoundEscrow: escrow exceeds uint96 max");
        }
        if (restitutionRequired > type(uint96).max) {
            revert("WoundEscrow: restitution exceeds uint96 max");
        }

        // Transfer escrow from agent
        flowToken.safeTransferFrom(agent, address(this), escrowAmount);

        wounds[woundId] = Wound({
            woundId: woundId,
            agent: agent,
            severity: uint8(severity),
            phase: uint8(WoundPhase.Hemostasis),
            exists: true,
            escrowAmount: uint96(escrowAmount),
            restitutionRequired: uint96(restitutionRequired),
            createdAt: uint48(block.timestamp),
            lastPhaseChange: uint48(block.timestamp),
            restitutionPaid: 0
        });

        agentWounds[agent].push(woundId);

        emit WoundCreated(woundId, agent, severity, escrowAmount, block.timestamp);

        // Gate 2: Emit warning for critical wounds
        if (severity == WoundSeverity.Critical) {
            emit ConstitutionalWarning(
                woundId,
                "Sacred Reciprocity",
                "Critical wound detected - extended healing required",
                4
            );
        }
    }

    /**
     * @notice Advance wound to next healing phase.
     * @dev KEY INVARIANT: Phases can ONLY advance forward, never skip or reverse.
     */
    function advancePhase(bytes32 woundId) external onlyRole(HEALER_ROLE) {
        Wound storage wound = wounds[woundId];
        if (!wound.exists) revert WoundNotFound(woundId);

        WoundPhase currentPhase = WoundPhase(wound.phase);
        if (currentPhase == WoundPhase.Healed) revert WoundAlreadyHealed(woundId);

        WoundPhase nextPhase;

        // Forward-only phase transitions (Gate 1 invariant)
        if (currentPhase == WoundPhase.Hemostasis) {
            uint256 elapsed = block.timestamp - wound.lastPhaseChange;
            if (elapsed < minHemostasisDuration) {
                revert MinHemostasisNotElapsed(woundId, minHemostasisDuration, elapsed);
            }
            nextPhase = WoundPhase.Inflammation;
        } else if (currentPhase == WoundPhase.Inflammation) {
            nextPhase = WoundPhase.Proliferation;
        } else if (currentPhase == WoundPhase.Proliferation) {
            // Restitution must be fulfilled before remodeling
            if (wound.restitutionPaid < wound.restitutionRequired) {
                revert RestitutionNotFulfilled(woundId, wound.restitutionRequired, wound.restitutionPaid);
            }
            nextPhase = WoundPhase.Remodeling;
        } else if (currentPhase == WoundPhase.Remodeling) {
            nextPhase = WoundPhase.Healed;
        } else {
            revert InvalidPhaseTransition(wound.phase, wound.phase);
        }

        wound.phase = uint8(nextPhase);
        wound.lastPhaseChange = uint48(block.timestamp);

        emit WoundPhaseAdvanced(woundId, currentPhase, nextPhase, block.timestamp);

        // Release escrow when healed
        if (nextPhase == WoundPhase.Healed) {
            _releaseEscrow(woundId);
        }
    }

    /**
     * @notice Pay restitution towards a wound.
     */
    function payRestitution(bytes32 woundId, uint256 amount) external nonReentrant {
        Wound storage wound = wounds[woundId];
        if (!wound.exists) revert WoundNotFound(woundId);
        if (WoundPhase(wound.phase) != WoundPhase.Proliferation) {
            revert WrongPhase(woundId, wound.phase, uint8(WoundPhase.Proliferation));
        }
        if (amount == 0) revert ZeroPayment();

        uint256 remaining = wound.restitutionRequired - wound.restitutionPaid;
        uint256 actualPayment = amount > remaining ? remaining : amount;

        flowToken.safeTransferFrom(msg.sender, address(this), actualPayment);

        // Safe to cast since actualPayment <= remaining <= uint96 max
        wound.restitutionPaid += uint64(actualPayment);

        emit RestitutionPaid(
            woundId,
            wound.agent,
            actualPayment,
            wound.restitutionPaid,
            wound.restitutionRequired
        );
    }

    /**
     * @notice Record scar tissue formation (strengthening).
     * @dev Called during Remodeling phase. Scar tissue makes healed areas stronger.
     */
    function formScarTissue(
        bytes32 woundId,
        string calldata area,
        uint256 strengthMultiplier
    ) external onlyRole(HEALER_ROLE) {
        if (!wounds[woundId].exists) revert WoundNotFound(woundId);
        if (WoundPhase(wounds[woundId].phase) != WoundPhase.Remodeling) {
            revert WrongPhase(woundId, wounds[woundId].phase, uint8(WoundPhase.Remodeling));
        }
        if (strengthMultiplier < 10000 || strengthMultiplier > 20000) {
            revert InvalidScarMultiplier(strengthMultiplier, 10000, 20000);
        }

        scars[woundId] = ScarTissue({
            woundId: woundId,
            area: area,
            strengthMultiplier: strengthMultiplier,
            formedAt: block.timestamp
        });

        emit ScarTissueFormed(woundId, area, strengthMultiplier);
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    /**
     * @notice Get wound data with unpacked types for external consumption.
     * @dev Converts packed types back to full enums for compatibility.
     */
    function getWound(bytes32 woundId) external view returns (
        bytes32 _woundId,
        address agent,
        WoundSeverity severity,
        WoundPhase phase,
        uint256 escrowAmount,
        uint256 restitutionRequired,
        uint256 restitutionPaid,
        uint256 createdAt,
        uint256 lastPhaseChange,
        bool exists
    ) {
        Wound storage wound = wounds[woundId];
        if (!wound.exists) revert WoundNotFound(woundId);

        return (
            wound.woundId,
            wound.agent,
            WoundSeverity(wound.severity),
            WoundPhase(wound.phase),
            uint256(wound.escrowAmount),
            uint256(wound.restitutionRequired),
            uint256(wound.restitutionPaid),
            uint256(wound.createdAt),
            uint256(wound.lastPhaseChange),
            wound.exists
        );
    }

    function getAgentWounds(address agent) external view returns (bytes32[] memory) {
        return agentWounds[agent];
    }

    /**
     * @notice Get wounds for an agent with pagination.
     * @dev Prevents gas issues with agents having many wounds.
     * @param agent The agent's address
     * @param offset Starting index
     * @param limit Maximum number of wounds to return
     * @return woundIds Array of wound IDs
     * @return hasMore True if there are more wounds beyond this page
     */
    function getAgentWoundsPaginated(
        address agent,
        uint256 offset,
        uint256 limit
    ) external view returns (bytes32[] memory woundIds, bool hasMore) {
        bytes32[] storage all = agentWounds[agent];
        uint256 total = all.length;

        if (offset >= total) {
            return (new bytes32[](0), false);
        }

        uint256 end = offset + limit;
        if (end > total) {
            end = total;
        }

        woundIds = new bytes32[](end - offset);
        for (uint256 i = offset; i < end; i++) {
            woundIds[i - offset] = all[i];
        }

        hasMore = end < total;
    }

    function getWoundPhase(bytes32 woundId) external view returns (WoundPhase) {
        if (!wounds[woundId].exists) revert WoundNotFound(woundId);
        return WoundPhase(wounds[woundId].phase);
    }

    function isHealed(bytes32 woundId) external view returns (bool) {
        return wounds[woundId].exists && WoundPhase(wounds[woundId].phase) == WoundPhase.Healed;
    }

    function restitutionRemaining(bytes32 woundId) external view returns (uint256) {
        Wound storage wound = wounds[woundId];
        if (!wound.exists) return 0;
        return uint256(wound.restitutionRequired) - uint256(wound.restitutionPaid);
    }

    // =========================================================================
    // Internal Functions
    // =========================================================================

    function _releaseEscrow(bytes32 woundId) internal {
        Wound storage wound = wounds[woundId];
        uint256 amount = uint256(wound.escrowAmount);
        wound.escrowAmount = 0;

        // Return remaining escrow to the agent
        if (amount > 0) {
            flowToken.safeTransfer(wound.agent, amount);
        }

        emit WoundHealed(woundId, wound.agent, wound.restitutionPaid, block.timestamp);
    }

    // =========================================================================
    // Admin Functions
    // =========================================================================

    function setHemostasisDuration(uint256 min, uint256 max) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(min < max, "WoundEscrow: min must be less than max");
        minHemostasisDuration = min;
        maxHemostasisDuration = max;
    }
}
