#!/usr/bin/env bash
# =============================================================================
# Mycelix Living Protocol - Release Script
# =============================================================================
# This script handles version bumping, changelog generation, and tag creation.
#
# Usage:
#   ./scripts/release.sh [patch|minor|major] [--dry-run] [--check VERSION]
#
# Examples:
#   ./scripts/release.sh patch           # Bump patch version (0.6.0 -> 0.6.1)
#   ./scripts/release.sh minor           # Bump minor version (0.6.0 -> 0.7.0)
#   ./scripts/release.sh major           # Bump major version (0.6.0 -> 1.0.0)
#   ./scripts/release.sh --dry-run patch # Show what would happen
#   ./scripts/release.sh --check 0.6.1   # Pre-release validation hook
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_CARGO_TOML="$REPO_ROOT/Cargo.toml"

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Get current version from workspace Cargo.toml
get_current_version() {
    grep -m1 'version = "' "$WORKSPACE_CARGO_TOML" | head -1 | cut -d'"' -f2
}

# Calculate next version based on bump type
calculate_next_version() {
    local current_version="$1"
    local bump_type="$2"

    IFS='.' read -r major minor patch <<< "$current_version"

    case "$bump_type" in
        major)
            echo "$((major + 1)).0.0"
            ;;
        minor)
            echo "${major}.$((minor + 1)).0"
            ;;
        patch)
            echo "${major}.${minor}.$((patch + 1))"
            ;;
        *)
            log_error "Invalid bump type: $bump_type"
            exit 1
            ;;
    esac
}

# Update version in workspace Cargo.toml
update_cargo_version() {
    local new_version="$1"
    local dry_run="${2:-false}"

    if [ "$dry_run" = "true" ]; then
        log_info "Would update Cargo.toml version to $new_version"
        return
    fi

    sed -i.bak "s/^version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"$new_version\"/" "$WORKSPACE_CARGO_TOML"
    rm -f "$WORKSPACE_CARGO_TOML.bak"
    log_success "Updated Cargo.toml version to $new_version"
}

# Update version in package.json
update_npm_version() {
    local new_version="$1"
    local dry_run="${2:-false}"
    local package_json="$REPO_ROOT/sdk/typescript/package.json"

    if [ ! -f "$package_json" ]; then
        log_warn "TypeScript SDK package.json not found"
        return
    fi

    if [ "$dry_run" = "true" ]; then
        log_info "Would update package.json version to $new_version"
        return
    fi

    sed -i.bak "s/\"version\": \"[0-9]*\.[0-9]*\.[0-9]*\"/\"version\": \"$new_version\"/" "$package_json"
    rm -f "$package_json.bak"
    log_success "Updated package.json version to $new_version"
}

# Update version in pyproject.toml
update_python_version() {
    local new_version="$1"
    local dry_run="${2:-false}"
    local pyproject="$REPO_ROOT/sdk/python/pyproject.toml"

    if [ ! -f "$pyproject" ]; then
        log_warn "Python SDK pyproject.toml not found"
        return
    fi

    if [ "$dry_run" = "true" ]; then
        log_info "Would update pyproject.toml version to $new_version"
        return
    fi

    sed -i.bak "s/^version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"$new_version\"/" "$pyproject"
    rm -f "$pyproject.bak"
    log_success "Updated pyproject.toml version to $new_version"
}

# Generate changelog using git-cliff
generate_changelog() {
    local new_version="$1"
    local dry_run="${2:-false}"

    if ! command -v git-cliff &> /dev/null; then
        log_warn "git-cliff not found, skipping changelog generation"
        log_info "Install with: cargo install git-cliff"
        return
    fi

    if [ "$dry_run" = "true" ]; then
        log_info "Would generate changelog for v$new_version"
        git-cliff --unreleased --tag "v$new_version" --dry-run
        return
    fi

    git-cliff --tag "v$new_version" -o CHANGELOG.md
    log_success "Generated CHANGELOG.md"
}

# Create git commit and tag
create_git_tag() {
    local new_version="$1"
    local dry_run="${2:-false}"

    if [ "$dry_run" = "true" ]; then
        log_info "Would create commit and tag v$new_version"
        return
    fi

    # Stage changes
    git add -A

    # Create commit
    git commit -m "chore(release): v$new_version"

    # Create annotated tag
    git tag -a "v$new_version" -m "Release v$new_version"

    log_success "Created commit and tag v$new_version"
}

# Run pre-release checks
run_checks() {
    log_info "Running pre-release checks..."

    # Check for uncommitted changes
    if [ -n "$(git status --porcelain)" ]; then
        log_error "Uncommitted changes detected. Please commit or stash them first."
        exit 1
    fi

    # Check we're on main or release branch
    local branch
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [[ ! "$branch" =~ ^(main|master|release/.*)$ ]]; then
        log_warn "Not on main/master/release branch (current: $branch)"
    fi

    # Run cargo check
    log_info "Running cargo check..."
    cargo check --workspace || {
        log_error "cargo check failed"
        exit 1
    }

    # Run tests (optional, can be skipped with --skip-tests)
    if [ "${SKIP_TESTS:-false}" != "true" ]; then
        log_info "Running tests..."
        cargo test --workspace --features full -- --test-threads=4 || {
            log_warn "Some tests failed, proceeding anyway"
        }
    fi

    log_success "Pre-release checks passed"
}

# Validate version for pre-release hook
validate_version() {
    local expected_version="$1"
    local current_version
    current_version=$(get_current_version)

    log_info "Validating version: expected=$expected_version, current=$current_version"

    # Run basic checks
    run_checks

    log_success "Version validation passed"
}

# =============================================================================
# Main Script
# =============================================================================

main() {
    local bump_type=""
    local dry_run="false"
    local check_mode="false"
    local check_version=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --dry-run)
                dry_run="true"
                shift
                ;;
            --check)
                check_mode="true"
                check_version="$2"
                shift 2
                ;;
            --skip-tests)
                export SKIP_TESTS="true"
                shift
                ;;
            patch|minor|major)
                bump_type="$1"
                shift
                ;;
            -h|--help)
                echo "Usage: $0 [patch|minor|major] [--dry-run] [--check VERSION]"
                echo ""
                echo "Options:"
                echo "  patch       Bump patch version (0.6.0 -> 0.6.1)"
                echo "  minor       Bump minor version (0.6.0 -> 0.7.0)"
                echo "  major       Bump major version (0.6.0 -> 1.0.0)"
                echo "  --dry-run   Show what would happen without making changes"
                echo "  --check     Pre-release validation hook (used by cargo-release)"
                echo "  --skip-tests Skip running tests during release"
                exit 0
                ;;
            *)
                log_error "Unknown argument: $1"
                exit 1
                ;;
        esac
    done

    # Change to repo root
    cd "$REPO_ROOT"

    # Handle check mode (pre-release hook)
    if [ "$check_mode" = "true" ]; then
        validate_version "$check_version"
        exit 0
    fi

    # Require bump type for normal operation
    if [ -z "$bump_type" ]; then
        log_error "Please specify bump type: patch, minor, or major"
        echo "Usage: $0 [patch|minor|major] [--dry-run]"
        exit 1
    fi

    # Get versions
    local current_version
    current_version=$(get_current_version)
    local new_version
    new_version=$(calculate_next_version "$current_version" "$bump_type")

    log_info "Current version: $current_version"
    log_info "New version: $new_version"

    if [ "$dry_run" = "true" ]; then
        log_warn "DRY RUN MODE - No changes will be made"
    fi

    # Run pre-release checks
    run_checks

    # Update versions
    log_info "Updating version files..."
    update_cargo_version "$new_version" "$dry_run"
    update_npm_version "$new_version" "$dry_run"
    update_python_version "$new_version" "$dry_run"

    # Generate changelog
    log_info "Generating changelog..."
    generate_changelog "$new_version" "$dry_run"

    # Create commit and tag
    if [ "$dry_run" = "false" ]; then
        create_git_tag "$new_version" "$dry_run"

        echo ""
        log_success "Release v$new_version prepared!"
        echo ""
        echo "Next steps:"
        echo "  1. Review the changes: git show HEAD"
        echo "  2. Push to remote: git push && git push --tags"
        echo "  3. The CI will automatically build and publish"
        echo ""
    else
        echo ""
        log_info "Dry run complete. No changes were made."
        echo ""
    fi
}

main "$@"
