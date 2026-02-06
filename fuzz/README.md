# Fuzzing for Mycelix v6.0 Living Protocol Layer

This directory contains fuzzing targets for the Living Protocol using [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) with libFuzzer.

## Prerequisites

1. Install Rust nightly (required for cargo-fuzz):
   ```bash
   rustup install nightly
   ```

2. Install cargo-fuzz:
   ```bash
   cargo +nightly install cargo-fuzz
   ```

## Available Fuzz Targets

| Target | Description |
|--------|-------------|
| `rpc_parse` | Fuzz JSON-RPC request parsing |
| `event_serialize` | Fuzz Living Protocol event serialization/deserialization |
| `cycle_state` | Fuzz cycle engine state machine operations |
| `kvector_operations` | Fuzz K-Vector mathematical operations |
| `beauty_scoring` | Fuzz beauty validity scoring engine |
| `wound_healing` | Fuzz wound healing state machine |

## Running Fuzz Targets

### Basic Usage

Run a fuzz target for 1 minute:
```bash
cd fuzz
cargo +nightly fuzz run rpc_parse -- -max_total_time=60
```

### Extended Fuzzing (1 hour)

For CI/scheduled runs:
```bash
cargo +nightly fuzz run rpc_parse -- -max_total_time=3600 -max_len=4096
```

### Running All Targets

```bash
for target in rpc_parse event_serialize cycle_state kvector_operations beauty_scoring wound_healing; do
    echo "Fuzzing $target..."
    cargo +nightly fuzz run $target -- -max_total_time=3600 -max_len=4096
done
```

## Configuration Options

| Option | Description | Recommended |
|--------|-------------|-------------|
| `-max_total_time=N` | Stop after N seconds | 3600 (1 hour) for CI |
| `-max_len=N` | Maximum input length in bytes | 4096-8192 |
| `-jobs=N` | Number of parallel jobs | CPU cores |
| `-dict=FILE` | Use a dictionary file | See dictionaries below |

## Corpus Management

Each target maintains a corpus directory under `fuzz/corpus/<target>/`.

### Minimizing Corpus

After extended fuzzing, minimize the corpus:
```bash
cargo +nightly fuzz cmin rpc_parse
```

### Sharing Corpus

The corpus is not checked into git. To share interesting inputs:
```bash
# Export interesting inputs
cp corpus/rpc_parse/*.input interesting_inputs/

# Import corpus from another machine
cp interesting_inputs/* corpus/rpc_parse/
```

## Crash Investigation

When a crash is found:

1. The crashing input is saved to `fuzz/artifacts/<target>/`

2. Reproduce the crash:
   ```bash
   cargo +nightly fuzz run rpc_parse -- artifacts/rpc_parse/crash-*
   ```

3. Minimize the crashing input:
   ```bash
   cargo +nightly fuzz tmin rpc_parse artifacts/rpc_parse/crash-abc123
   ```

4. Debug with full stacktrace:
   ```bash
   RUST_BACKTRACE=1 cargo +nightly fuzz run rpc_parse -- artifacts/rpc_parse/crash-abc123
   ```

## Dictionaries

Create dictionaries to help the fuzzer with structured input:

### `dict/json.dict`
```
"{"
"}"
"["
"]"
":"
","
"\""
"null"
"true"
"false"
"id"
"method"
"params"
```

Use with:
```bash
cargo +nightly fuzz run rpc_parse -- -dict=dict/json.dict
```

## CI Integration

See `.github/workflows/fuzzing.yml` for automated weekly fuzzing runs.

### Running in Docker

```bash
docker run --rm -v $(pwd):/work -w /work/fuzz \
    rustlang/rust:nightly \
    cargo fuzz run rpc_parse -- -max_total_time=3600
```

## Coverage

Generate coverage reports to identify areas needing more fuzzing:

```bash
# Build with coverage
cargo +nightly fuzz coverage rpc_parse

# View coverage report
llvm-cov show -format=html \
    target/x86_64-unknown-linux-gnu/coverage/x86_64-unknown-linux-gnu/release/rpc_parse \
    -instr-profile=coverage/rpc_parse/coverage.profdata \
    > coverage.html
```

## Writing New Fuzz Targets

1. Create a new file in `fuzz/fuzz_targets/`:
   ```rust
   #![no_main]
   use libfuzzer_sys::fuzz_target;

   fuzz_target!(|data: &[u8]| {
       // Your fuzzing logic here
   });
   ```

2. Add the target to `fuzz/Cargo.toml`:
   ```toml
   [[bin]]
   name = "my_target"
   path = "fuzz_targets/my_target.rs"
   test = false
   doc = false
   bench = false
   ```

3. Test it works:
   ```bash
   cargo +nightly fuzz run my_target -- -max_total_time=60
   ```

## Troubleshooting

### "error: could not compile `cc`"

Install build dependencies:
```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install
```

### "error: failed to run custom build command for `openssl-sys`"

Install OpenSSL development files:
```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# macOS
brew install openssl
```

### Out of Memory

Limit memory usage:
```bash
cargo +nightly fuzz run rpc_parse -- -rss_limit_mb=2048
```
