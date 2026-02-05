# Contributing to Mycelix v6.0 Living Protocol

Thank you for your interest in contributing to the Mycelix Living Protocol Layer!

## Getting Started

### Prerequisites

- Rust 1.75+ with `wasm32-unknown-unknown` target
- Node.js 18+ and npm
- Git

### Setup

```bash
# Clone the repository
git clone https://github.com/mycelix/mycelix-v6-living.git
cd mycelix-v6-living

# Build the project
cargo build --workspace --features full

# Run tests
cargo test --workspace --features full

# Build TypeScript SDK
cd sdk/typescript && npm install && npm run build
```

## Development Workflow

### Branch Naming

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions/improvements

### Commit Messages

Follow conventional commits format:

```
type(scope): description

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Examples:
```
feat(metabolism): add wound healing progress tracking
fix(cycle-engine): correct phase transition timing
docs(readme): update installation instructions
```

### Pull Request Process

1. Fork the repository
2. Create a feature branch from `develop`
3. Make your changes
4. Run tests: `cargo test --workspace --features full`
5. Run clippy: `cargo clippy --workspace --features full`
6. Run formatter: `cargo fmt --all`
7. Push and create a PR against `develop`

## Code Guidelines

### Rust

- Use `rustfmt` for formatting
- Address all `clippy` warnings
- Add documentation comments for public APIs
- Write tests for new functionality
- Follow the existing code patterns

### TypeScript

- Use TypeScript strict mode
- Add JSDoc comments for public APIs
- Write tests for new functionality

### Solidity

- Follow Solidity style guide
- Add NatSpec documentation
- Write Foundry tests for new contracts

## Testing

### Running Tests

```bash
# All Rust tests
cargo test --workspace --features full

# Specific crate
cargo test -p metabolism

# With output
cargo test --workspace --features full -- --nocapture

# TypeScript SDK
cd sdk/typescript && npm test

# Solidity (requires Foundry)
forge test
```

### Writing Tests

- Unit tests go in the same file as the code (`#[cfg(test)]` module)
- Integration tests go in `tests/` directory
- Property-based tests use `proptest`
- Benchmark tests use `criterion`

## Architecture

### Crate Structure

```
crates/
├── living-core/      # Shared types, events, errors
├── metabolism/       # Primitives 1-4
├── consciousness/    # Primitives 5-8
├── epistemics/       # Primitives 9-12
├── relational/       # Primitives 13-16
├── structural/       # Primitives 17-21
└── cycle-engine/     # Orchestration
```

### Adding a New Primitive

1. Add types to `living-core/src/types.rs`
2. Add events to `living-core/src/events.rs`
3. Implement engine in appropriate crate
4. Add phase handler in `cycle-engine`
5. Add zome entry types and functions
6. Add SDK bindings
7. Add tests

### Gate System

When adding validation logic:

- **Gate 1**: Hard invariants (blocking) - Must not violate
- **Gate 2**: Soft constraints (warning) - Should log warnings
- **Gate 3**: Network health (advisory) - Informational

## Documentation

- Update README.md for user-facing changes
- Update ARCHITECTURE.md for structural changes
- Add inline documentation with `///` comments
- Update CHANGELOG.md for releases

## Release Process

1. Update version in `Cargo.toml` (workspace)
2. Update `CHANGELOG.md`
3. Create a PR to `main`
4. After merge, tag the release: `git tag v0.x.0`
5. Push tags: `git push --tags`
6. CI will build and publish

## Getting Help

- Open an issue for bugs or feature requests
- Join discussions in GitHub Discussions
- Read the [Architecture Docs](./docs/ARCHITECTURE.md)

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/).
Be respectful, inclusive, and constructive.

## License

By contributing, you agree that your contributions will be licensed under AGPL-3.0-or-later.
