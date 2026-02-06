---
sidebar_position: 100
title: Contributing
---

# Contributing to Mycelix

Thank you for your interest in contributing to Mycelix! This guide will help you get started.

## Code of Conduct

We are committed to providing a welcoming and inspiring community for all. Please read and follow our [Code of Conduct](https://github.com/mycelix/mycelix/blob/main/CODE_OF_CONDUCT.md).

## Ways to Contribute

### Report Bugs

Found a bug? Please [open an issue](https://github.com/mycelix/mycelix/issues/new?template=bug_report.md) with:

- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, Node version, etc.)
- Current cycle phase (if relevant)

### Suggest Features

Have an idea? [Open a feature request](https://github.com/mycelix/mycelix/issues/new?template=feature_request.md) with:

- Clear description of the feature
- Use cases and motivation
- Proposed implementation (if you have one)
- How it aligns with the cycle philosophy

### Improve Documentation

Documentation improvements are always welcome:

- Fix typos and errors
- Add examples and clarifications
- Translate to other languages
- Write tutorials and guides

### Contribute Code

Ready to code? Here's how:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## Development Setup

### Prerequisites

- Node.js 20+
- Rust 1.75+ (for core)
- Docker (for integration tests)

### Clone and Install

```bash
# Clone the repository
git clone https://github.com/mycelix/mycelix.git
cd mycelix

# Install dependencies
npm install

# Build all packages
npm run build

# Run tests
npm test
```

### Project Structure

```
mycelix/
├── core/               # Rust core library
├── server/             # Node.js server
├── packages/
│   ├── core/           # TypeScript core
│   ├── sdk/            # TypeScript SDK
│   ├── react/          # React bindings
│   └── testing/        # Test utilities
├── sdks/
│   ├── python/         # Python SDK
│   └── go/             # Go SDK
├── docs-site/          # Documentation
└── examples/           # Example projects
```

### Development Commands

```bash
# Start development server
npm run dev

# Run specific package tests
npm run test -w @mycelix/core

# Run linting
npm run lint

# Format code
npm run format

# Type check
npm run typecheck
```

## Pull Request Process

### Before Submitting

1. **Check existing issues** - Someone may already be working on it
2. **Open an issue first** - For significant changes, discuss before coding
3. **Follow the style guide** - Consistent code is easier to review
4. **Write tests** - All new features need tests
5. **Update documentation** - Keep docs in sync with code

### PR Guidelines

**Title format:**

```
<type>(<scope>): <description>

Examples:
feat(primitives): add new Rhythm primitive
fix(cycle): correct phase transition timing
docs(sdk): add Python async examples
```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation only
- `style` - Formatting, no code change
- `refactor` - Code change that neither fixes nor adds
- `perf` - Performance improvement
- `test` - Adding tests
- `chore` - Maintenance

**Description template:**

```markdown
## Summary
Brief description of changes.

## Changes
- Change 1
- Change 2

## Testing
How was this tested?

## Phase Considerations
Does this change behave differently across phases?

## Breaking Changes
Any breaking changes? Migration steps?
```

### Review Process

1. **Automated checks** - CI must pass
2. **Code review** - At least one maintainer approval
3. **Documentation review** - For user-facing changes
4. **Phase testing** - Verify behavior across all phases

## Coding Standards

### TypeScript

```typescript
// Use explicit types
function calculate(value: number): number {
  return value * 2;
}

// Prefer const and immutability
const config = Object.freeze({
  timeout: 5000,
});

// Use async/await over promises
async function fetch() {
  const result = await api.get('/data');
  return result;
}

// Document public APIs
/**
 * Invokes a primitive with the given payload.
 * @param id - Primitive identifier
 * @param payload - Data to send
 * @returns Invocation result
 */
export async function invoke(id: string, payload: unknown): Promise<Result> {
  // ...
}
```

### Rust

```rust
// Follow Rust conventions
pub fn calculate(value: i32) -> i32 {
    value * 2
}

// Use Result for fallible operations
pub fn parse(input: &str) -> Result<Config, ParseError> {
    // ...
}

// Document public items
/// Invokes a primitive with the given payload.
///
/// # Arguments
/// * `id` - Primitive identifier
/// * `payload` - Data to send
///
/// # Returns
/// Invocation result or error
pub fn invoke(id: &str, payload: Value) -> Result<Value, Error> {
    // ...
}
```

### Python

```python
# Use type hints
def calculate(value: int) -> int:
    return value * 2

# Use dataclasses for data
@dataclass
class Config:
    timeout: int = 5000

# Use async for IO
async def fetch() -> Result:
    result = await api.get("/data")
    return result

# Document with docstrings
def invoke(id: str, payload: Any) -> Result:
    """Invoke a primitive with the given payload.

    Args:
        id: Primitive identifier
        payload: Data to send

    Returns:
        Invocation result
    """
    ...
```

## Testing Guidelines

### Unit Tests

Test individual functions and methods:

```typescript
describe('Pulse', () => {
  it('should emit at correct interval', async () => {
    const pulse = new Pulse({ interval: '1s' });
    const emissions: any[] = [];

    pulse.on('emit', (data) => emissions.push(data));
    await pulse.start();
    await sleep(2500);
    await pulse.stop();

    expect(emissions.length).toBe(2);
  });
});
```

### Integration Tests

Test component interactions:

```typescript
describe('Client-Server', () => {
  let server: MycelixServer;
  let client: MycelixClient;

  beforeAll(async () => {
    server = await createServer({ port: 0 });
    client = new MycelixClient({ url: server.wsUrl });
    await client.connect();
  });

  afterAll(async () => {
    await client.disconnect();
    await server.close();
  });

  it('should invoke primitives', async () => {
    const result = await client.primitives.invoke('test-thread', { action: 'echo' });
    expect(result.action).toBe('echo');
  });
});
```

### Phase Tests

Test behavior across phases:

```typescript
describe('Phase-aware behavior', () => {
  const phases = ['Dawn', 'Surge', 'Settle', 'Rest'] as const;

  for (const phase of phases) {
    it(`should behave correctly in ${phase}`, async () => {
      const client = createTestClient({ phase });
      const result = await client.primitives.invoke('adaptive-thread', {});

      expect(result.processedDuring).toBe(phase);
    });
  }
});
```

## Documentation Guidelines

### Writing Style

- Use **active voice**: "Mycelix provides" not "is provided by"
- Be **concise**: Avoid filler words
- Use **examples**: Show, don't just tell
- Be **precise**: Specific details over vague descriptions

### Code Examples

- Include **complete, runnable examples**
- Show **imports** and setup
- Add **comments** for complex parts
- Test examples before submitting

### API Documentation

- Document all **public APIs**
- Include **parameter descriptions**
- Show **return values**
- Note **exceptions/errors**
- Add **usage examples**

## Release Process

### Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Release Schedule

- **Patches**: As needed
- **Minor**: Monthly (targeting Dawn phase)
- **Major**: Quarterly (targeting Dawn phase)

## Getting Help

- **Discord**: [Join our server](https://discord.gg/mycelix)
- **GitHub Discussions**: [Ask questions](https://github.com/mycelix/mycelix/discussions)
- **Stack Overflow**: Tag `mycelix`

## Recognition

Contributors are recognized in:

- CONTRIBUTORS.md file
- Release notes
- Project website

Thank you for contributing to Mycelix!
