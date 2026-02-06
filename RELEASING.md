# Release Process

This document describes how to create releases for the Mycelix Living Protocol.

## Table of Contents

- [Versioning Policy](#versioning-policy)
- [Release Types](#release-types)
- [Release Checklist](#release-checklist)
- [Automated Release](#automated-release)
- [Manual Release](#manual-release)
- [Package Publishing](#package-publishing)
- [Troubleshooting](#troubleshooting)

## Versioning Policy

This project follows [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** version when making incompatible API changes
- **MINOR** version when adding functionality in a backward compatible manner
- **PATCH** version when making backward compatible bug fixes

### Version Format

```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]
```

Examples:
- `0.6.0` - Initial v6 release
- `0.6.1` - Patch release with bug fixes
- `0.7.0` - Minor release with new features
- `1.0.0` - First stable major release
- `1.0.0-alpha.1` - Pre-release version
- `1.0.0-beta.1` - Beta pre-release
- `1.0.0-rc.1` - Release candidate

### Pre-release Versions

| Type | Purpose | Example |
|------|---------|---------|
| `alpha` | Early testing, may have bugs | `0.7.0-alpha.1` |
| `beta` | Feature complete, testing | `0.7.0-beta.1` |
| `rc` | Release candidate | `0.7.0-rc.1` |

## Release Types

### Patch Release (0.6.x)

Use for:
- Bug fixes
- Security patches
- Documentation updates
- Dependency updates (non-breaking)

```bash
./scripts/release.sh patch
```

### Minor Release (0.x.0)

Use for:
- New features
- Non-breaking API additions
- Performance improvements
- Deprecations

```bash
./scripts/release.sh minor
```

### Major Release (x.0.0)

Use for:
- Breaking API changes
- Major architectural changes
- Removal of deprecated features

```bash
./scripts/release.sh major
```

## Release Checklist

Before starting a release, complete this checklist:

### Pre-Release

- [ ] All tests pass: `cargo test --workspace --features full`
- [ ] No clippy warnings: `cargo clippy --workspace --features full`
- [ ] Code formatted: `cargo fmt --all --check`
- [ ] TypeScript SDK tests pass: `cd sdk/typescript && npm test`
- [ ] Python SDK tests pass: `cd sdk/python && pytest`
- [ ] Go SDK tests pass: `cd sdk/go && go test ./...`
- [ ] Documentation is up to date
- [ ] CHANGELOG.md entries reviewed
- [ ] No uncommitted changes in working directory
- [ ] On `main` or `release/*` branch

### Version Consistency

- [ ] Workspace `Cargo.toml` version updated
- [ ] `sdk/typescript/package.json` version matches
- [ ] `sdk/python/pyproject.toml` version matches
- [ ] `sdk/go/go.mod` module path is correct

### Post-Release

- [ ] Release tag created and pushed
- [ ] GitHub Release published
- [ ] Crates published to crates.io
- [ ] npm package published
- [ ] PyPI package published
- [ ] Go module tagged
- [ ] Release notes reviewed
- [ ] Announce release (if applicable)

## Automated Release

The recommended way to create a release is using the automated workflow:

### 1. Prepare Release

```bash
# Ensure you're on main branch
git checkout main
git pull origin main

# Run the release script
./scripts/release.sh patch  # or minor/major

# Review changes
git show HEAD
git log --oneline -5
```

### 2. Push Release

```bash
# Push commits and tags
git push origin main
git push origin --tags
```

### 3. Monitor CI

Once the tag is pushed, GitHub Actions will:

1. Validate the release tag
2. Build binaries for all platforms
3. Build Holochain DNA
4. Build and push Docker image
5. Generate changelog
6. Create GitHub Release with assets
7. Publish to crates.io, npm, PyPI
8. Tag Go module

Monitor progress at: https://github.com/mycelix/mycelix-v6-living/actions

## Manual Release

If automation fails, you can publish manually:

### Rust Crates

```bash
# Set your crates.io token
export CARGO_REGISTRY_TOKEN=your_token

# Publish in dependency order
cargo publish -p living-core
sleep 45
cargo publish -p metabolism
cargo publish -p consciousness
cargo publish -p epistemics
cargo publish -p relational
cargo publish -p structural
sleep 45
cargo publish -p cycle-engine
```

### TypeScript SDK

```bash
cd sdk/typescript
npm login
npm publish --access public
```

### Python SDK

```bash
cd sdk/python
python -m build
twine upload dist/*
```

### Go SDK

```bash
# Create subtree tag for Go module
git tag sdk/go/v0.6.0
git push origin sdk/go/v0.6.0
```

### Docker Image

```bash
docker build -t ghcr.io/mycelix/mycelix-v6-living:0.6.0 .
docker push ghcr.io/mycelix/mycelix-v6-living:0.6.0
```

## Package Publishing

### crates.io

Rust crates are published in dependency order:

1. `living-core` - Core types and events
2. `metabolism`, `consciousness`, `epistemics`, `relational`, `structural` - Domain crates
3. `cycle-engine` - Orchestration engine

**Required:** `CARGO_REGISTRY_TOKEN` secret

### npm

The TypeScript SDK is published to npm as `@mycelix/living-protocol-sdk`.

**Required:** `NPM_TOKEN` secret

### PyPI

The Python SDK is published to PyPI as `mycelix`.

**Required:** `PYPI_API_TOKEN` secret

### pkg.go.dev

The Go SDK is automatically indexed by pkg.go.dev when tagged.

Module path: `github.com/mycelix/mycelix-go`

## Troubleshooting

### Release Script Fails

```bash
# Check for uncommitted changes
git status

# Check current branch
git branch

# Run with dry-run to see what would happen
./scripts/release.sh --dry-run patch
```

### crates.io Publish Fails

Common issues:
- Crate already exists with same version
- Missing required metadata in Cargo.toml
- Dependencies not yet available

```bash
# Check what would be published
cargo publish -p living-core --dry-run

# Verify crate metadata
cargo package -p living-core --list
```

### npm Publish Fails

Common issues:
- Not logged in
- Package version already exists
- Missing build step

```bash
# Verify you're logged in
npm whoami

# Check what would be published
npm pack --dry-run

# Build before publishing
npm run build
```

### Docker Build Fails

```bash
# Build locally to debug
docker build --progress=plain -t mycelix-test .

# Check for platform issues
docker buildx build --platform linux/amd64,linux/arm64 .
```

### Changelog Generation Issues

```bash
# Verify git-cliff is installed
git-cliff --version

# Generate changelog preview
git-cliff --unreleased

# Check cliff.toml configuration
git-cliff --config cliff.toml
```

## Release Artifacts

Each release produces:

| Artifact | Location |
|----------|----------|
| Linux x64 binary | `mycelix-living-linux-x64.tar.gz` |
| Linux arm64 binary | `mycelix-living-linux-arm64.tar.gz` |
| macOS x64 binary | `mycelix-living-macos-x64.tar.gz` |
| macOS arm64 binary | `mycelix-living-macos-arm64.tar.gz` |
| Windows x64 binary | `mycelix-living-windows-x64.zip` |
| Windows arm64 binary | `mycelix-living-windows-arm64.zip` |
| Holochain DNA | `mycelix-living-protocol.dna` |
| Holochain hApp | `mycelix-living-protocol.happ` |
| Docker image | `ghcr.io/mycelix/mycelix-v6-living:VERSION` |
| Checksums | `SHA256SUMS.txt` |

## Support

If you encounter issues:

1. Check the [GitHub Actions logs](https://github.com/mycelix/mycelix-v6-living/actions)
2. Review this documentation
3. Open an issue with the `release` label
