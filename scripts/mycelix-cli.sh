#!/usr/bin/env bash
#
# Mycelix v6.0 Living Protocol CLI
#
# A command-line tool for local development, testing, and debugging.
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored message
print_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
print_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Show usage
usage() {
    cat << EOF
Mycelix v6.0 Living Protocol CLI

USAGE:
    mycelix-cli <COMMAND> [OPTIONS]

COMMANDS:
    test        Run tests
    build       Build the project
    bench       Run benchmarks
    check       Check code (fmt, clippy, tests)
    zome        Holochain zome commands
    contract    Solidity contract commands
    sdk         TypeScript SDK commands
    cycle       Cycle engine utilities
    help        Show this help message

EXAMPLES:
    mycelix-cli test                    Run all Rust tests
    mycelix-cli test --features full    Run tests with all features
    mycelix-cli build --release         Build release binaries
    mycelix-cli zome build              Build Holochain zomes
    mycelix-cli contract test           Run Solidity tests
    mycelix-cli sdk build               Build TypeScript SDK
    mycelix-cli cycle info              Show cycle information

EOF
}

# Run tests
cmd_test() {
    local features="${1:-}"

    print_info "Running tests..."

    if [[ -n "$features" ]]; then
        cargo test --workspace --features "$features"
    else
        cargo test --workspace --features full
    fi

    print_success "All tests passed!"
}

# Build project
cmd_build() {
    local release="${1:-}"

    print_info "Building project..."

    if [[ "$release" == "--release" ]]; then
        cargo build --release --workspace --features full
    else
        cargo build --workspace --features full
    fi

    print_success "Build complete!"
}

# Run benchmarks
cmd_bench() {
    print_info "Running benchmarks..."

    cargo bench --features full

    print_success "Benchmarks complete!"
}

# Check code quality
cmd_check() {
    print_info "Checking code quality..."

    print_info "Checking formatting..."
    cargo fmt --all -- --check

    print_info "Running clippy..."
    cargo clippy --workspace --features full -- -D warnings

    print_info "Running tests..."
    cargo test --workspace --features full

    print_success "All checks passed!"
}

# Holochain zome commands
cmd_zome() {
    local subcmd="${1:-help}"

    case "$subcmd" in
        build)
            print_info "Building Holochain zomes..."

            if ! command -v rustup &> /dev/null; then
                print_error "rustup not found. Please install Rust."
                exit 1
            fi

            # Add WASM target if not present
            rustup target add wasm32-unknown-unknown 2>/dev/null || true

            # Build each zome
            for zome_dir in "$PROJECT_ROOT"/zomes/*/coordinator "$PROJECT_ROOT"/zomes/*/integrity "$PROJECT_ROOT"/zomes/shared; do
                if [[ -f "$zome_dir/Cargo.toml" ]]; then
                    print_info "Building $(basename "$(dirname "$zome_dir")")/$(basename "$zome_dir")..."
                    cargo build --release --target wasm32-unknown-unknown --manifest-path "$zome_dir/Cargo.toml" || {
                        print_warning "Failed to build $zome_dir (may need Holochain SDK)"
                    }
                fi
            done

            print_success "Zome build complete!"
            ;;

        pack)
            print_info "Packing DNA..."

            if ! command -v hc &> /dev/null; then
                print_error "hc (Holochain CLI) not found. Please install via holonix."
                exit 1
            fi

            hc dna pack "$PROJECT_ROOT/dna/"
            hc app pack "$PROJECT_ROOT/"

            print_success "DNA packed!"
            ;;

        *)
            echo "Zome commands: build, pack"
            ;;
    esac
}

# Solidity contract commands
cmd_contract() {
    local subcmd="${1:-help}"

    case "$subcmd" in
        build)
            print_info "Building Solidity contracts..."

            if ! command -v forge &> /dev/null; then
                print_error "forge not found. Please install Foundry."
                exit 1
            fi

            cd "$PROJECT_ROOT"
            forge build

            print_success "Contract build complete!"
            ;;

        test)
            print_info "Running Solidity tests..."

            if ! command -v forge &> /dev/null; then
                print_error "forge not found. Please install Foundry."
                exit 1
            fi

            cd "$PROJECT_ROOT"
            forge test -vvv

            print_success "Contract tests complete!"
            ;;

        coverage)
            print_info "Running coverage..."

            cd "$PROJECT_ROOT"
            forge coverage --report summary

            print_success "Coverage complete!"
            ;;

        *)
            echo "Contract commands: build, test, coverage"
            ;;
    esac
}

# TypeScript SDK commands
cmd_sdk() {
    local subcmd="${1:-help}"

    case "$subcmd" in
        build)
            print_info "Building TypeScript SDK..."

            cd "$PROJECT_ROOT/sdk/typescript"
            npm install
            npm run build

            print_success "SDK build complete!"
            ;;

        test)
            print_info "Running SDK tests..."

            cd "$PROJECT_ROOT/sdk/typescript"
            npm test

            print_success "SDK tests complete!"
            ;;

        *)
            echo "SDK commands: build, test"
            ;;
    esac
}

# Cycle engine utilities
cmd_cycle() {
    local subcmd="${1:-info}"

    case "$subcmd" in
        info)
            cat << 'EOF'
=== Mycelix v6.0 - 28-Day Metabolism Cycle ===

Phase                  | Days | Duration | Operations
-----------------------|------|----------|---------------------------
Shadow                 | 1-3  | 3 days   | Surface suppressed content
Composting             | 4-6  | 3 days   | Decompose failed patterns
Liminal                | 7-9  | 3 days   | Threshold transitions
Negative Capability    | 10-12| 3 days   | Hold in uncertainty
Eros                   | 13-15| 3 days   | Attractor field activation
Co-Creation            | 16-18| 3 days   | Entanglement formation
Beauty                 | 19-20| 2 days   | Aesthetic validation
Emergent Personhood    | 21-24| 4 days   | Phi measurement
Kenosis                | 25-28| 4 days   | Self-emptying commitments

Total: 28 days (lunar cycle)

Gate System:
- Gate 1: Hard invariants (blocking)
- Gate 2: Soft constraints (warning)
- Gate 3: Network health (advisory)

EOF
            ;;

        simulate)
            print_info "Running cycle simulation..."

            cargo test --package cycle-engine test_full_cycle_with_wired_handlers -- --nocapture

            print_success "Simulation complete!"
            ;;

        *)
            echo "Cycle commands: info, simulate"
            ;;
    esac
}

# Main entry point
main() {
    local cmd="${1:-help}"
    shift || true

    case "$cmd" in
        test)
            cmd_test "$@"
            ;;
        build)
            cmd_build "$@"
            ;;
        bench)
            cmd_bench "$@"
            ;;
        check)
            cmd_check "$@"
            ;;
        zome)
            cmd_zome "$@"
            ;;
        contract)
            cmd_contract "$@"
            ;;
        sdk)
            cmd_sdk "$@"
            ;;
        cycle)
            cmd_cycle "$@"
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            print_error "Unknown command: $cmd"
            usage
            exit 1
            ;;
    esac
}

main "$@"
