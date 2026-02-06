// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../../contracts/WoundEscrow.sol";
import "../../contracts/KenosisBurn.sol";
import "../../contracts/FractalDAO.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";

/**
 * @title Gas Benchmarks
 * @notice Measures gas consumption for key operations across Mycelix contracts.
 * @dev Run with: forge test --match-path test/gas/*.sol --gas-report
 *
 * Benchmark Categories:
 *   - WoundEscrow: Creation, phase advancement, restitution
 *   - KenosisBurn: Commitment, execution
 *   - FractalDAO: Pattern creation, replication, proposals
 */

/// @dev Mock ERC20 for gas testing.
contract GasTestToken is ERC20 {
    constructor() ERC20("GasTestToken", "GAS") {
        _mint(msg.sender, 10_000_000 ether);
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

/// @dev Mock burnable ERC20 for kenosis testing.
contract GasTestBurnableToken is ERC20, ERC20Burnable {
    constructor() ERC20("GasBurnableToken", "GBURN") {
        _mint(msg.sender, 10_000_000 ether);
    }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract WoundEscrowGasBenchmark is Test {
    WoundEscrow public escrow;
    GasTestToken public token;

    address public admin = address(this);
    address public agent = address(0xBEEF);
    address public healer = address(0xCAFE);

    function setUp() public {
        token = new GasTestToken();
        escrow = new WoundEscrow(address(token));

        escrow.grantRole(escrow.HEALER_ROLE(), healer);

        token.mint(agent, 1_000_000 ether);
        vm.prank(agent);
        token.approve(address(escrow), type(uint256).max);
    }

    // =========================================================================
    // Creation Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for creating a wound.
    function test_gas_createWound() public {
        bytes32 woundId = keccak256("gas-wound-1");

        uint256 gasBefore = gasleft();
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Minor,
            1 ether,
            0.5 ether
        );
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("createWound gas", gasUsed);
    }

    /// @notice Benchmark gas for creating a critical wound (emits extra event).
    function test_gas_createWound_critical() public {
        bytes32 woundId = keccak256("gas-wound-critical");

        uint256 gasBefore = gasleft();
        escrow.createWound(
            woundId,
            agent,
            WoundEscrow.WoundSeverity.Critical,
            1 ether,
            0.5 ether
        );
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("createWound (critical) gas", gasUsed);
    }

    // =========================================================================
    // Phase Advancement Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for advancing from Hemostasis to Inflammation.
    function test_gas_advancePhase_hemostasis() public {
        bytes32 woundId = keccak256("gas-advance-1");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 0);
        vm.warp(block.timestamp + 2 hours);

        vm.prank(healer);
        uint256 gasBefore = gasleft();
        escrow.advancePhase(woundId);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("advancePhase (Hemostasis->Inflammation) gas", gasUsed);
    }

    /// @notice Benchmark gas for advancing to Healed (triggers escrow release).
    function test_gas_advancePhase_toHealed() public {
        bytes32 woundId = keccak256("gas-advance-healed");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 0);

        vm.warp(block.timestamp + 2 hours);
        vm.startPrank(healer);
        escrow.advancePhase(woundId); // -> Inflammation
        escrow.advancePhase(woundId); // -> Proliferation
        escrow.advancePhase(woundId); // -> Remodeling

        uint256 gasBefore = gasleft();
        escrow.advancePhase(woundId); // -> Healed (releases escrow)
        uint256 gasUsed = gasBefore - gasleft();
        vm.stopPrank();

        emit log_named_uint("advancePhase (Remodeling->Healed, with escrow release) gas", gasUsed);
    }

    // =========================================================================
    // Restitution Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for paying restitution.
    function test_gas_payRestitution() public {
        bytes32 woundId = keccak256("gas-restitution");
        escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Moderate, 5 ether, 2 ether);

        vm.warp(block.timestamp + 2 hours);
        vm.startPrank(healer);
        escrow.advancePhase(woundId); // -> Inflammation
        escrow.advancePhase(woundId); // -> Proliferation
        vm.stopPrank();

        token.mint(agent, 2 ether);

        vm.prank(agent);
        uint256 gasBefore = gasleft();
        escrow.payRestitution(woundId, 1 ether);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("payRestitution gas", gasUsed);
    }

    // =========================================================================
    // View Function Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for getAgentWounds with varying wound counts.
    function test_gas_getAgentWounds_scaling() public {
        // Create multiple wounds
        for (uint256 i = 0; i < 10; i++) {
            bytes32 woundId = keccak256(abi.encodePacked("gas-scale-", i));
            token.mint(agent, 1 ether);
            escrow.createWound(woundId, agent, WoundEscrow.WoundSeverity.Minor, 1 ether, 0);
        }

        uint256 gasBefore = gasleft();
        bytes32[] memory wounds = escrow.getAgentWounds(agent);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("getAgentWounds (10 wounds) gas", gasUsed);
        assertEq(wounds.length, 10);
    }
}

contract KenosisBurnGasBenchmark is Test {
    KenosisBurn public kenosis;
    GasTestBurnableToken public token;

    address public admin = address(this);
    address public agent = address(0xBEEF);

    function setUp() public {
        token = new GasTestBurnableToken();
        kenosis = new KenosisBurn(address(token));

        token.mint(agent, 1_000_000 ether);
        vm.prank(agent);
        token.approve(address(kenosis), type(uint256).max);
    }

    // =========================================================================
    // Commitment Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for committing kenosis.
    function test_gas_commitKenosis() public {
        bytes32 commitmentId = keccak256("gas-kenosis-1");

        vm.prank(agent);
        uint256 gasBefore = gasleft();
        kenosis.commitKenosis(commitmentId, 1000); // 10%
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("commitKenosis gas", gasUsed);
    }

    /// @notice Benchmark gas for committing at max percentage (20%).
    function test_gas_commitKenosis_maxRelease() public {
        bytes32 commitmentId = keccak256("gas-kenosis-max");

        vm.prank(agent);
        uint256 gasBefore = gasleft();
        kenosis.commitKenosis(commitmentId, 2000); // 20% max
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("commitKenosis (max 20%) gas", gasUsed);
    }

    // =========================================================================
    // Execution Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for executing kenosis (burn).
    function test_gas_executeKenosis() public {
        bytes32 commitmentId = keccak256("gas-execute");

        vm.prank(agent);
        kenosis.commitKenosis(commitmentId, 1000);

        vm.prank(agent);
        uint256 gasBefore = gasleft();
        kenosis.executeKenosis(commitmentId);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("executeKenosis (with ERC20Burnable) gas", gasUsed);
    }

    // =========================================================================
    // Cycle Advancement Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for advancing cycle.
    function test_gas_advanceCycle() public {
        uint256 gasBefore = gasleft();
        kenosis.advanceCycle();
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("advanceCycle gas", gasUsed);
    }

    // =========================================================================
    // View Function Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for getRemainingCycleCapacity.
    function test_gas_getRemainingCycleCapacity() public {
        uint256 gasBefore = gasleft();
        uint256 capacity = kenosis.getRemainingCycleCapacity(agent);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("getRemainingCycleCapacity gas", gasUsed);
        assertEq(capacity, 2000); // Full capacity
    }
}

contract FractalDAOGasBenchmark is Test {
    FractalDAO public dao;

    address public admin = address(this);
    address public voter = address(0xBEEF);

    function setUp() public {
        dao = new FractalDAO();
    }

    // =========================================================================
    // Pattern Creation Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for creating a pattern.
    function test_gas_createPattern() public {
        bytes32 patternId = keccak256("gas-pattern-1");

        uint256 gasBefore = gasleft();
        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Community,
            5000, // 50% quorum
            6667, // 66.67% supermajority
            FractalDAO.DecisionMechanism.Supermajority
        );
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("createPattern gas", gasUsed);
    }

    // =========================================================================
    // Pattern Replication Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for replicating to child scale.
    function test_gas_replicateToChildScale() public {
        bytes32 parentId = keccak256("gas-parent");
        bytes32 childId = keccak256("gas-child");

        dao.createPattern(
            parentId,
            FractalDAO.GovernanceScale.Community,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Supermajority
        );

        uint256 gasBefore = gasleft();
        dao.replicateToChildScale(parentId, childId);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("replicateToChildScale gas", gasUsed);
    }

    /// @notice Benchmark gas for replicating to parent scale.
    function test_gas_replicateToParentScale() public {
        bytes32 childId = keccak256("gas-child-2");
        bytes32 parentId = keccak256("gas-parent-2");

        dao.createPattern(
            childId,
            FractalDAO.GovernanceScale.Team,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Supermajority
        );

        uint256 gasBefore = gasleft();
        dao.replicateToParentScale(childId, parentId);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("replicateToParentScale gas", gasUsed);
    }

    // =========================================================================
    // Proposal Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for creating a proposal.
    function test_gas_createProposal() public {
        bytes32 patternId = keccak256("gas-pattern-proposal");
        bytes32 proposalId = keccak256("gas-proposal-1");

        dao.createPattern(
            patternId,
            FractalDAO.GovernanceScale.Team,
            5000,
            6667,
            FractalDAO.DecisionMechanism.Supermajority
        );

        uint256 gasBefore = gasleft();
        dao.createProposal(proposalId, patternId, "Test proposal for gas benchmark", 100, 1 days);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("createProposal gas", gasUsed);
    }

    /// @notice Benchmark gas for voting.
    function test_gas_vote() public {
        bytes32 patternId = keccak256("gas-pattern-vote");
        bytes32 proposalId = keccak256("gas-proposal-vote");

        dao.createPattern(patternId, FractalDAO.GovernanceScale.Team, 5000, 6667, FractalDAO.DecisionMechanism.Supermajority);
        dao.createProposal(proposalId, patternId, "Vote benchmark", 100, 1 days);

        vm.prank(voter);
        uint256 gasBefore = gasleft();
        dao.vote(proposalId, true);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("vote gas", gasUsed);
    }

    /// @notice Benchmark gas for resolving a proposal.
    function test_gas_resolveProposal() public {
        bytes32 patternId = keccak256("gas-pattern-resolve");
        bytes32 proposalId = keccak256("gas-proposal-resolve");

        dao.createPattern(patternId, FractalDAO.GovernanceScale.Team, 5000, 6667, FractalDAO.DecisionMechanism.Supermajority);
        dao.createProposal(proposalId, patternId, "Resolve benchmark", 10, 1);

        // Cast some votes
        for (uint160 i = 1; i <= 8; i++) {
            vm.prank(address(i));
            dao.vote(proposalId, true);
        }

        vm.warp(block.timestamp + 2);

        uint256 gasBefore = gasleft();
        dao.resolveProposal(proposalId);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("resolveProposal gas", gasUsed);
    }

    // =========================================================================
    // Structural Identity Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for verifying structural identity.
    function test_gas_verifyStructuralIdentity() public {
        bytes32 patternA = keccak256("gas-identity-a");
        bytes32 patternB = keccak256("gas-identity-b");

        dao.createPattern(patternA, FractalDAO.GovernanceScale.Team, 5000, 6667, FractalDAO.DecisionMechanism.Supermajority);
        dao.createPattern(patternB, FractalDAO.GovernanceScale.Community, 5000, 6667, FractalDAO.DecisionMechanism.Supermajority);

        uint256 gasBefore = gasleft();
        bool identical = dao.verifyStructuralIdentity(patternA, patternB);
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("verifyStructuralIdentity gas", gasUsed);
        assertTrue(identical);
    }

    // =========================================================================
    // View Function Scaling Benchmarks
    // =========================================================================

    /// @notice Benchmark gas for getAllPatterns with varying counts.
    function test_gas_getAllPatterns_scaling() public {
        // Create multiple patterns
        for (uint256 i = 0; i < 10; i++) {
            bytes32 patternId = keccak256(abi.encodePacked("gas-scale-pattern-", i));
            dao.createPattern(
                patternId,
                FractalDAO.GovernanceScale.Team,
                5000,
                6667,
                FractalDAO.DecisionMechanism.Supermajority
            );
        }

        uint256 gasBefore = gasleft();
        bytes32[] memory patterns = dao.getAllPatterns();
        uint256 gasUsed = gasBefore - gasleft();

        emit log_named_uint("getAllPatterns (10 patterns) gas", gasUsed);
        assertEq(patterns.length, 10);
    }
}

// =============================================================================
// Summary Test
// =============================================================================

contract GasBenchmarkSummary is Test {
    /// @notice Placeholder for running all benchmarks.
    function test_gas_summary() public {
        emit log("=== Gas Benchmark Summary ===");
        emit log("Run: forge test --match-path test/gas/*.sol --gas-report -vvv");
        emit log("");
        emit log("Key operations to optimize:");
        emit log("- WoundEscrow.createWound: High-frequency operation");
        emit log("- KenosisBurn.commitKenosis: Storage-heavy");
        emit log("- FractalDAO.createPattern: Creates multiple storage entries");
        emit log("");
        emit log("Optimization targets:");
        emit log("- Custom errors: ~2000 gas per revert");
        emit log("- Struct packing: ~60,000 gas for large structs");
        emit log("- Pagination: Prevents unbounded loops");
    }
}
