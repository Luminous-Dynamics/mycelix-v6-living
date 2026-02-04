// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

/**
 * @title WoundEscrow
 * @notice On-chain restitution tracking for wound healing [Primitive 2].
 * @dev Replaces punitive slashing with a healing-oriented escrow mechanism.
 *      Funds are escrowed during hemostasis and released as restitution is fulfilled.
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

    struct Wound {
        bytes32 woundId;
        address agent;
        WoundSeverity severity;
        WoundPhase phase;
        uint256 escrowAmount;
        uint256 restitutionRequired;
        uint256 restitutionPaid;
        uint256 createdAt;
        uint256 lastPhaseChange;
        bool exists;
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
        require(!wounds[woundId].exists, "WoundEscrow: wound already exists");
        require(agent != address(0), "WoundEscrow: zero address");
        require(escrowAmount > 0, "WoundEscrow: zero escrow");

        // Transfer escrow from agent
        flowToken.safeTransferFrom(agent, address(this), escrowAmount);

        wounds[woundId] = Wound({
            woundId: woundId,
            agent: agent,
            severity: severity,
            phase: WoundPhase.Hemostasis,
            escrowAmount: escrowAmount,
            restitutionRequired: restitutionRequired,
            restitutionPaid: 0,
            createdAt: block.timestamp,
            lastPhaseChange: block.timestamp,
            exists: true
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
        require(wound.exists, "WoundEscrow: wound not found");
        require(wound.phase != WoundPhase.Healed, "WoundEscrow: already healed");

        WoundPhase currentPhase = wound.phase;
        WoundPhase nextPhase;

        // Forward-only phase transitions (Gate 1 invariant)
        if (currentPhase == WoundPhase.Hemostasis) {
            require(
                block.timestamp >= wound.lastPhaseChange + minHemostasisDuration,
                "WoundEscrow: minimum hemostasis not elapsed"
            );
            nextPhase = WoundPhase.Inflammation;
        } else if (currentPhase == WoundPhase.Inflammation) {
            nextPhase = WoundPhase.Proliferation;
        } else if (currentPhase == WoundPhase.Proliferation) {
            // Restitution must be fulfilled before remodeling
            require(
                wound.restitutionPaid >= wound.restitutionRequired,
                "WoundEscrow: restitution not fulfilled"
            );
            nextPhase = WoundPhase.Remodeling;
        } else if (currentPhase == WoundPhase.Remodeling) {
            nextPhase = WoundPhase.Healed;
        } else {
            revert("WoundEscrow: invalid phase");
        }

        wound.phase = nextPhase;
        wound.lastPhaseChange = block.timestamp;

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
        require(wound.exists, "WoundEscrow: wound not found");
        require(wound.phase == WoundPhase.Proliferation, "WoundEscrow: not in proliferation phase");
        require(amount > 0, "WoundEscrow: zero amount");

        uint256 remaining = wound.restitutionRequired - wound.restitutionPaid;
        uint256 actualPayment = amount > remaining ? remaining : amount;

        flowToken.safeTransferFrom(msg.sender, address(this), actualPayment);
        wound.restitutionPaid += actualPayment;

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
        require(wounds[woundId].exists, "WoundEscrow: wound not found");
        require(
            wounds[woundId].phase == WoundPhase.Remodeling,
            "WoundEscrow: not in remodeling phase"
        );
        require(strengthMultiplier >= 10000, "WoundEscrow: multiplier must be >= 1.0x");
        require(strengthMultiplier <= 20000, "WoundEscrow: multiplier must be <= 2.0x");

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

    function getWound(bytes32 woundId) external view returns (Wound memory) {
        require(wounds[woundId].exists, "WoundEscrow: wound not found");
        return wounds[woundId];
    }

    function getAgentWounds(address agent) external view returns (bytes32[] memory) {
        return agentWounds[agent];
    }

    function getWoundPhase(bytes32 woundId) external view returns (WoundPhase) {
        require(wounds[woundId].exists, "WoundEscrow: wound not found");
        return wounds[woundId].phase;
    }

    function isHealed(bytes32 woundId) external view returns (bool) {
        return wounds[woundId].exists && wounds[woundId].phase == WoundPhase.Healed;
    }

    function restitutionRemaining(bytes32 woundId) external view returns (uint256) {
        Wound storage wound = wounds[woundId];
        if (!wound.exists) return 0;
        return wound.restitutionRequired - wound.restitutionPaid;
    }

    // =========================================================================
    // Internal Functions
    // =========================================================================

    function _releaseEscrow(bytes32 woundId) internal {
        Wound storage wound = wounds[woundId];
        uint256 amount = wound.escrowAmount;
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
