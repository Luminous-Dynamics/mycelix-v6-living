# Mycelix v6.0 Living Protocol - Deployment Guide

This guide covers the complete deployment process for the Mycelix v6.0 Living Protocol Layer, including Holochain DNA deployment, Solidity contract deployment, and integration configuration.

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Holochain DNA Deployment](#2-holochain-dna-deployment)
3. [Solidity Contract Deployment](#3-solidity-contract-deployment)
4. [Integration Configuration](#4-integration-configuration)
5. [Operational Procedures](#5-operational-procedures)
6. [Security Considerations](#6-security-considerations)

---

## 1. Prerequisites

### 1.1 Holochain Conductor Requirements

**Software Requirements:**

- **Holochain**: v0.4.x or later (compatible with HDK 0.6.0)
- **Nix package manager**: For holonix development environment
- **Rust toolchain**: 1.75+ with wasm32-unknown-unknown target
- **hc CLI**: Holochain command-line tools

**Installation:**

```bash
# Install Nix (if not already installed)
curl -L https://nixos.org/nix/install | sh

# Enter holonix development shell
nix develop github:holochain/holochain#holonix

# Verify installation
hc --version
holochain --version
```

**Required Rust target:**

```bash
rustup target add wasm32-unknown-unknown
```

### 1.2 EVM-Compatible Blockchain Requirements

**Supported Networks:**

- Ethereum Mainnet
- Sepolia Testnet (recommended for testing)
- Goerli Testnet (deprecated, migration recommended)
- Polygon, Arbitrum, Optimism (L2 options)

**Software Requirements:**

- **Foundry**: v0.2.0+ (forge, cast, anvil)
- **Node.js**: v18+ (for deployment scripts)
- **Solidity**: 0.8.20 (specified in `foundry.toml`)

**Installation:**

```bash
# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Verify installation
forge --version
cast --version
```

### 1.3 Infrastructure Requirements

**Hardware (Production):**

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 8+ cores |
| RAM | 8 GB | 16+ GB |
| Storage | 100 GB SSD | 500 GB NVMe SSD |
| Network | 100 Mbps | 1 Gbps |

**Networking:**

- **Holochain Conductor Ports:**
  - Admin interface: 4444 (internal only)
  - App interface: 8888 (WebSocket)
  - P2P network: 5678 (UDP, configurable)

- **Monitoring Ports:**
  - Prometheus: 9090
  - Grafana: 3000
  - Metrics exporter: 9100

**Firewall Rules:**

```bash
# Allow Holochain P2P (adjust port as needed)
ufw allow 5678/udp

# Allow WebSocket app interface (consider restricting source IPs)
ufw allow 8888/tcp

# Block admin interface from external access
ufw deny 4444/tcp
```

---

## 2. Holochain DNA Deployment

### 2.1 Building WASM Zomes

The Living Protocol consists of six zome pairs (integrity + coordinator):

- `living_metabolism` - Composting, wound healing, kenosis, metabolic trust
- `living_consciousness` - K-vector, field interference, collective dreaming
- `living_epistemics` - Shadow integration, negative capability, beauty validity
- `living_relational` - Entangled pairs, eros attractor, liminality
- `living_structural` - Resonance addressing, fractal governance, morphogenetic fields
- `bridge` - v5.2 integration layer

**Build all zomes:**

```bash
# From project root, enter holonix environment
nix develop

# Build all zomes for WASM target
cargo build --release --target wasm32-unknown-unknown

# Verify WASM files exist
ls -la target/wasm32-unknown-unknown/release/*.wasm
```

**Expected output files:**

```
target/wasm32-unknown-unknown/release/
├── living_metabolism_integrity.wasm
├── living_metabolism.wasm
├── living_consciousness_integrity.wasm
├── living_consciousness.wasm
├── living_epistemics_integrity.wasm
├── living_epistemics.wasm
├── living_relational_integrity.wasm
├── living_relational.wasm
├── living_structural_integrity.wasm
├── living_structural.wasm
├── bridge_integrity.wasm
└── bridge.wasm
```

### 2.2 Creating DNA Bundle

The DNA manifest is defined in `dna/dna.yaml`:

```yaml
---
manifest_version: "1"
name: mycelix-living-protocol
integrity:
  origin_time: "2025-01-01T00:00:00.000000Z"
  network_seed: ~
  zomes:
    - name: living_metabolism_integrity
      bundled: "../target/wasm32-unknown-unknown/release/living_metabolism_integrity.wasm"
    # ... (all integrity zomes)
coordinator:
  zomes:
    - name: living_metabolism
      bundled: "../target/wasm32-unknown-unknown/release/living_metabolism.wasm"
      dependencies:
        - name: living_metabolism_integrity
    # ... (all coordinator zomes)
```

**Package the DNA:**

```bash
# Create DNA bundle
hc dna pack dna/ -o dna/mycelix-living-protocol.dna

# Verify DNA hash
hc dna hash dna/mycelix-living-protocol.dna
```

### 2.3 Creating hApp Bundle

The hApp manifest is defined in `happ.yaml`:

```yaml
---
manifest_version: "1"
name: mycelix-living-protocol
description: "Mycelix v6.0 Living Protocol Layer hApp"
roles:
  - name: mycelix_living
    provisioning:
      strategy: create
      deferred: false
    dna:
      bundled: "dna/mycelix-living-protocol.dna"
      modifiers:
        network_seed: ~
        properties: ~
        origin_time: ~
        quantum_time: ~
```

**Package the hApp:**

```bash
# Create hApp bundle
hc app pack . -o mycelix-living-protocol.happ

# Verify bundle
hc app hash mycelix-living-protocol.happ
```

### 2.4 Configuring Conductor

Create a conductor configuration file `conductor-config.yaml`:

```yaml
---
environment_path: /var/lib/holochain/conductor
keystore:
  type: lair_server
  connection_url: "unix:///var/run/lair-keystore/socket?k=<lair_pubkey>"

admin_interfaces:
  - driver:
      type: websocket
      port: 4444
    allowed_origins: "*"

network:
  transport_pool:
    - type: webrtc
      signal_url: "wss://signal.holochain.org"
  bootstrap_service: "https://bootstrap.holochain.org"
  tuning_params:
    gossip_loop_iteration_delay_ms: 1000
    default_rpc_single_timeout_ms: 60000

# Production: Set appropriate values
dpki:
  instance_id: ~
  init_remote_key: ~
```

### 2.5 Installing DNA in Conductor

**Start the conductor:**

```bash
# Start Holochain conductor with configuration
holochain -c conductor-config.yaml
```

**Install the hApp via admin interface:**

```bash
# Generate agent key
hc sandbox generate

# Install hApp
hc sandbox call install-app \
  --app-id mycelix-living-protocol \
  --agent-key <agent_pub_key> \
  --path mycelix-living-protocol.happ
```

**Programmatic installation (Node.js):**

```typescript
import { AdminWebsocket, AppWebsocket } from '@holochain/client';

const adminWs = await AdminWebsocket.connect('ws://localhost:4444');

// Generate agent key
const agentKey = await adminWs.generateAgentPubKey();

// Install hApp
const appInfo = await adminWs.installApp({
  installed_app_id: 'mycelix-living-protocol',
  agent_key: agentKey,
  path: './mycelix-living-protocol.happ',
});

// Enable the app
await adminWs.enableApp({ installed_app_id: appInfo.installed_app_id });
```

### 2.6 Testing with hc sandbox

**Create a sandbox environment for testing:**

```bash
# Create sandbox with 2 conductors
hc sandbox create -n 2 -d mycelix-test

# Run sandbox
hc sandbox run mycelix-test

# In another terminal, generate test data
hc sandbox call mycelix-test 0 living_metabolism create_wound ...
```

**Run integration tests:**

```bash
# Run Holochain integration tests (requires holonix)
cargo test --release --features mock_hdk
```

---

## 3. Solidity Contract Deployment

### 3.1 Pre-deployment Checklist

Before deploying contracts, verify:

- [ ] All contracts compile without errors: `forge build`
- [ ] All tests pass: `forge test`
- [ ] Fuzz tests pass: `forge test --fuzz-runs 1000`
- [ ] Gas estimates are acceptable: `forge test --gas-report`
- [ ] Contract sizes are within limits: `forge build --sizes`
- [ ] OpenZeppelin dependencies are up-to-date
- [ ] Private keys are securely stored (hardware wallet or secure enclave)
- [ ] Deployment wallet has sufficient ETH for gas
- [ ] Flow token (for WoundEscrow) and reputation token (for KenosisBurn) addresses are known

**Configuration in `foundry.toml`:**

```toml
[profile.default]
src = "contracts"
out = "out"
libs = ["lib"]
solc = "0.8.20"

[profile.default.fuzz]
runs = 256
```

### 3.2 Deploying to Testnet (Sepolia)

**Environment setup:**

```bash
# Create .env file (DO NOT COMMIT)
cat > .env << 'EOF'
PRIVATE_KEY=0x...
SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/<project_id>
ETHERSCAN_API_KEY=<api_key>
FLOW_TOKEN_ADDRESS=0x...
REPUTATION_TOKEN_ADDRESS=0x...
EOF

# Load environment
source .env
```

**Deploy WoundEscrow:**

```bash
forge create contracts/WoundEscrow.sol:WoundEscrow \
  --rpc-url $SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY \
  --constructor-args $FLOW_TOKEN_ADDRESS \
  --verify \
  --etherscan-api-key $ETHERSCAN_API_KEY
```

**Deploy KenosisBurn:**

```bash
forge create contracts/KenosisBurn.sol:KenosisBurn \
  --rpc-url $SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY \
  --constructor-args $REPUTATION_TOKEN_ADDRESS \
  --verify \
  --etherscan-api-key $ETHERSCAN_API_KEY
```

**Deploy FractalDAO:**

```bash
forge create contracts/FractalDAO.sol:FractalDAO \
  --rpc-url $SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY \
  --verify \
  --etherscan-api-key $ETHERSCAN_API_KEY
```

### 3.3 Deploying to Mainnet

**Additional precautions for mainnet:**

1. **Use a hardware wallet** (Ledger/Trezor)
2. **Simulate deployment first:**

```bash
# Dry run deployment
forge script script/Deploy.s.sol \
  --rpc-url $MAINNET_RPC_URL \
  --private-key $PRIVATE_KEY \
  --slow \
  --broadcast=false
```

3. **Deploy with hardware wallet:**

```bash
forge script script/Deploy.s.sol \
  --rpc-url $MAINNET_RPC_URL \
  --ledger \
  --sender <ledger_address> \
  --broadcast \
  --verify
```

4. **Record deployment addresses** in a secure location

### 3.4 Verifying Contracts on Etherscan

If verification was not done during deployment:

```bash
# Verify WoundEscrow
forge verify-contract \
  --chain-id 1 \
  --compiler-version 0.8.20 \
  --constructor-args $(cast abi-encode "constructor(address)" $FLOW_TOKEN_ADDRESS) \
  <deployed_address> \
  contracts/WoundEscrow.sol:WoundEscrow \
  --etherscan-api-key $ETHERSCAN_API_KEY

# Verify KenosisBurn
forge verify-contract \
  --chain-id 1 \
  --compiler-version 0.8.20 \
  --constructor-args $(cast abi-encode "constructor(address)" $REPUTATION_TOKEN_ADDRESS) \
  <deployed_address> \
  contracts/KenosisBurn.sol:KenosisBurn \
  --etherscan-api-key $ETHERSCAN_API_KEY

# Verify FractalDAO (no constructor args)
forge verify-contract \
  --chain-id 1 \
  --compiler-version 0.8.20 \
  <deployed_address> \
  contracts/FractalDAO.sol:FractalDAO \
  --etherscan-api-key $ETHERSCAN_API_KEY
```

### 3.5 Setting Up Contract Roles (AccessControl)

After deployment, configure access control roles:

**WoundEscrow Roles:**

```bash
# Role hashes
HEALER_ROLE=$(cast keccak "HEALER_ROLE")
VALIDATOR_ROLE=$(cast keccak "VALIDATOR_ROLE")

# Grant HEALER_ROLE to healer address
cast send $WOUND_ESCROW_ADDRESS \
  "grantRole(bytes32,address)" \
  $HEALER_ROLE \
  $HEALER_ADDRESS \
  --rpc-url $RPC_URL \
  --private-key $ADMIN_PRIVATE_KEY

# Grant VALIDATOR_ROLE to validator address
cast send $WOUND_ESCROW_ADDRESS \
  "grantRole(bytes32,address)" \
  $VALIDATOR_ROLE \
  $VALIDATOR_ADDRESS \
  --rpc-url $RPC_URL \
  --private-key $ADMIN_PRIVATE_KEY
```

**KenosisBurn Roles:**

```bash
CYCLE_MANAGER_ROLE=$(cast keccak "CYCLE_MANAGER_ROLE")

# Grant CYCLE_MANAGER_ROLE to cycle manager
cast send $KENOSIS_BURN_ADDRESS \
  "grantRole(bytes32,address)" \
  $CYCLE_MANAGER_ROLE \
  $CYCLE_MANAGER_ADDRESS \
  --rpc-url $RPC_URL \
  --private-key $ADMIN_PRIVATE_KEY
```

**Renounce admin role (for decentralization):**

```bash
# WARNING: This is irreversible
DEFAULT_ADMIN_ROLE=0x0000000000000000000000000000000000000000000000000000000000000000

cast send $CONTRACT_ADDRESS \
  "renounceRole(bytes32,address)" \
  $DEFAULT_ADMIN_ROLE \
  $ADMIN_ADDRESS \
  --rpc-url $RPC_URL \
  --private-key $ADMIN_PRIVATE_KEY
```

---

## 4. Integration Configuration

### 4.1 Connecting TypeScript SDK to Holochain

Install the SDK:

```bash
npm install @mycelix/living-protocol-sdk @holochain/client
```

**Initialize connection:**

```typescript
import { AppWebsocket } from '@holochain/client';
import {
  MetabolismClient,
  ConsciousnessClient,
  EpistemicsClient,
  RelationalClient,
  StructuralClient
} from '@mycelix/living-protocol-sdk';

// Connect to Holochain app interface
const appWs = await AppWebsocket.connect('ws://localhost:8888');

// Get app info
const appInfo = await appWs.appInfo({
  installed_app_id: 'mycelix-living-protocol'
});

const cellId = appInfo.cell_info.mycelix_living[0].cell_id;

// Initialize module clients
const metabolism = new MetabolismClient(appWs, cellId, 'living_metabolism');
const consciousness = new ConsciousnessClient(appWs, cellId, 'living_consciousness');
const epistemics = new EpistemicsClient(appWs, cellId, 'living_epistemics');
const relational = new RelationalClient(appWs, cellId, 'living_relational');
const structural = new StructuralClient(appWs, cellId, 'living_structural');

// Example: Create a wound
const woundRecord = await metabolism.createWound({
  agent: agentPubKey,
  severity: 'moderate',
  cause: 'trust_violation',
  restitutionRequired: 1000n,
});
```

### 4.2 Connecting to EVM Contracts

**Using ethers.js:**

```typescript
import { ethers } from 'ethers';

// ABIs (generated by forge build)
import WoundEscrowABI from '../out/WoundEscrow.sol/WoundEscrow.json';
import KenosisBurnABI from '../out/KenosisBurn.sol/KenosisBurn.json';
import FractalDAOABI from '../out/FractalDAO.sol/FractalDAO.json';

// Connect to provider
const provider = new ethers.JsonRpcProvider(process.env.RPC_URL);
const signer = new ethers.Wallet(process.env.PRIVATE_KEY, provider);

// Initialize contracts
const woundEscrow = new ethers.Contract(
  process.env.WOUND_ESCROW_ADDRESS,
  WoundEscrowABI.abi,
  signer
);

const kenosisBurn = new ethers.Contract(
  process.env.KENOSIS_BURN_ADDRESS,
  KenosisBurnABI.abi,
  signer
);

const fractalDAO = new ethers.Contract(
  process.env.FRACTAL_DAO_ADDRESS,
  FractalDAOABI.abi,
  signer
);

// Example: Create wound on-chain
const woundId = ethers.keccak256(ethers.toUtf8Bytes('wound-001'));
const tx = await woundEscrow.createWound(
  woundId,
  agentAddress,
  1, // WoundSeverity.Moderate
  ethers.parseEther('100'), // escrowAmount
  ethers.parseEther('50')   // restitutionRequired
);
await tx.wait();
```

### 4.3 Bridge Zome Configuration for v5.2 Integration

The bridge zome enables cross-DNA communication with the v5.2 Property DNA:

**Bridge zome functions:**

| Function | Description |
|----------|-------------|
| `fetch_matl_score` | Fetch MATL score from v5.2 |
| `intercept_slash` | Convert slash to wound healing |
| `fetch_k_vector_snapshot` | Get K-vector from v5.2 |
| `attach_beauty_score` | Attach beauty score to proposal |
| `resolve_did` | Resolve DID via v5.2 agent registry |

**Configuration in conductor:**

To enable cross-DNA calls, both DNAs must be installed in the same conductor with appropriate capabilities:

```yaml
# In happ.yaml for v5.2 integration
roles:
  - name: mycelix_living
    dna:
      bundled: "dna/mycelix-living-protocol.dna"
  - name: mycelix_property
    dna:
      path: "path/to/mycelix-property.dna"
      # Or use network reference
      network_seed: "<v5.2_network_seed>"
```

**TypeScript bridge usage:**

```typescript
import { BridgeClient } from '@mycelix/living-protocol-sdk';

const bridge = new BridgeClient(appWs, cellId, 'bridge');

// Fetch MATL score from v5.2
const matlScore = await bridge.fetchMatlScore({
  agent: agentPubKey,
});

// Intercept a slash event
const wound = await bridge.interceptSlash({
  offender: agentPubKey,
  slashPercentage: 15,
  originalActionHash: actionHash,
});

// Get migration status
const status = await bridge.getMigrationStatus();
console.log(`Wounds created: ${status.woundsCreated}`);
console.log(`Wounds healed: ${status.woundsHealed}`);
```

---

## 5. Operational Procedures

### 5.1 Monitoring Setup (Prometheus/Grafana)

**Prometheus Configuration:**

Create `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "monitoring/prometheus-rules.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  - job_name: 'holochain'
    static_configs:
      - targets: ['localhost:9100']
    metrics_path: /metrics

  - job_name: 'mycelix-living'
    static_configs:
      - targets: ['localhost:9101']
```

**Key metrics from `monitoring/prometheus-rules.yml`:**

| Metric | Description |
|--------|-------------|
| `mycelix_cycle_number` | Current 28-day cycle |
| `mycelix_current_phase` | Active phase (0-8) |
| `mycelix_phase_day` | Day within phase (1-28) |
| `mycelix_active_wounds` | Wounds in healing |
| `mycelix_healed_wounds` | Completed healings |
| `mycelix_network_phi` | Integrated information |
| `mycelix_metabolic_trust` | Trust scores |

**Grafana Dashboard:**

Import the dashboard from `monitoring/grafana-dashboard.json`:

1. Open Grafana (http://localhost:3000)
2. Go to Dashboards > Import
3. Upload `grafana-dashboard.json`
4. Select Prometheus data source

The dashboard includes:
- Cycle overview (current cycle, phase, day)
- Wound healing & composting metrics
- K-vector dimensions visualization
- Network Phi (integrated information)
- Shadow integration tracking
- Gate system alerts

### 5.2 Log Aggregation

**Structured logging configuration:**

```bash
# Set Holochain log level
export RUST_LOG=info,holochain=debug,mycelix=trace

# Start conductor with JSON logging
holochain -c conductor-config.yaml 2>&1 | \
  jq -c 'select(.level != "trace")' >> /var/log/holochain/conductor.jsonl
```

**Loki/Promtail configuration:**

```yaml
# promtail-config.yaml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: holochain
    static_configs:
      - targets:
          - localhost
        labels:
          job: holochain
          __path__: /var/log/holochain/*.jsonl
    pipeline_stages:
      - json:
          expressions:
            level: level
            module: target
      - labels:
          level:
          module:
```

### 5.3 Backup Procedures

**Holochain data backup:**

```bash
#!/bin/bash
# backup-holochain.sh

BACKUP_DIR="/backups/holochain/$(date +%Y%m%d)"
DATA_DIR="/var/lib/holochain/conductor"

mkdir -p "$BACKUP_DIR"

# Stop conductor gracefully
systemctl stop holochain-conductor

# Backup lair keystore
cp -r /var/lib/lair-keystore "$BACKUP_DIR/lair-keystore"

# Backup conductor data
tar -czf "$BACKUP_DIR/conductor-data.tar.gz" "$DATA_DIR"

# Restart conductor
systemctl start holochain-conductor

# Upload to remote storage (S3, GCS, etc.)
aws s3 sync "$BACKUP_DIR" s3://mycelix-backups/holochain/
```

**EVM state backup:**

For contract state, rely on blockchain archival nodes. For off-chain data:

```bash
# Export event logs
cast logs \
  --from-block <deployment_block> \
  --to-block latest \
  --address $WOUND_ESCROW_ADDRESS \
  --rpc-url $RPC_URL \
  > wound_events.json
```

### 5.4 Incident Response

**Severity Levels:**

| Level | Description | Response Time |
|-------|-------------|---------------|
| P1 | Gate 1 violation, system down | 15 minutes |
| P2 | Gate 2 warnings, degraded service | 1 hour |
| P3 | Gate 3 advisories, minor issues | 24 hours |

**Gate 1 Violation Response:**

Gate 1 violations indicate hard invariant failures. See `monitoring/prometheus-rules.yml` for alert definitions:

```yaml
- alert: Gate1Violation
  expr: increase(gate1_violation_total[5m]) > 0
  for: 0s
  labels:
    severity: critical
```

Response procedure:
1. Immediately investigate the violation
2. Pause affected operations if necessary
3. Do not override Gate 1 protections
4. Document root cause
5. Deploy fix with thorough testing

**Runbook locations:**

- Holochain conductor issues: `docs/runbooks/holochain.md`
- EVM contract emergencies: `docs/runbooks/evm-emergency.md`
- Bridge integration failures: `docs/runbooks/bridge.md`

---

## 6. Security Considerations

### 6.1 Key Management

**Holochain Keys:**

- Agent keys are managed by Lair Keystore
- Never expose the lair socket outside localhost
- Use hardware security modules (HSM) in production

```bash
# Lair keystore security
chmod 700 /var/lib/lair-keystore
chown holochain:holochain /var/lib/lair-keystore
```

**EVM Private Keys:**

- Use hardware wallets for all admin operations
- Never store private keys in environment variables on production servers
- Use AWS KMS, HashiCorp Vault, or similar for programmatic access

```bash
# Example: Using HashiCorp Vault
export VAULT_ADDR='https://vault.mycelix.io:8200'
vault kv get -field=private_key secret/mycelix/deployer
```

**Multisig for Admin Functions:**

Deploy a Gnosis Safe multisig for admin role:

```bash
# Transfer DEFAULT_ADMIN_ROLE to multisig
cast send $CONTRACT_ADDRESS \
  "grantRole(bytes32,address)" \
  $DEFAULT_ADMIN_ROLE \
  $GNOSIS_SAFE_ADDRESS \
  --rpc-url $RPC_URL \
  --private-key $ADMIN_PRIVATE_KEY

# Renounce from EOA
cast send $CONTRACT_ADDRESS \
  "renounceRole(bytes32,address)" \
  $DEFAULT_ADMIN_ROLE \
  $ADMIN_ADDRESS \
  --rpc-url $RPC_URL \
  --private-key $ADMIN_PRIVATE_KEY
```

### 6.2 Network Isolation

**Holochain Network Segmentation:**

```yaml
# conductor-config.yaml
network:
  # Use private bootstrap server for production
  bootstrap_service: "https://bootstrap.mycelix-internal.io"

  # Restrict P2P connections
  tuning_params:
    gossip_peer_on_success_next_gossip_delay_ms: 5000
    gossip_peer_on_error_next_gossip_delay_ms: 30000
```

**Infrastructure isolation:**

```
┌─────────────────────────────────────────────────────────────┐
│                        VPC / Private Network                 │
├─────────────────────────────────────────────────────────────┤
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │ Holochain  │  │ Holochain  │  │ Holochain  │            │
│  │ Conductor  │  │ Conductor  │  │ Conductor  │            │
│  │    (P2P)   │  │    (P2P)   │  │    (P2P)   │            │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘            │
│        │               │               │                    │
│        └───────────────┼───────────────┘                    │
│                        │                                    │
│                  ┌─────┴─────┐                              │
│                  │  Internal │                              │
│                  │    LB     │                              │
│                  └─────┬─────┘                              │
│                        │                                    │
├────────────────────────┼────────────────────────────────────┤
│                  ┌─────┴─────┐                              │
│                  │    WAF    │     Public Zone              │
│                  └─────┬─────┘                              │
│                        │                                    │
│                  ┌─────┴─────┐                              │
│                  │ API Gateway│                              │
│                  └───────────┘                              │
└─────────────────────────────────────────────────────────────┘
```

### 6.3 Rate Limiting

**Holochain zome rate limiting:**

Implement rate limiting within zome code:

```rust
// Example rate limiting pattern
const MAX_CALLS_PER_MINUTE: u32 = 60;

fn check_rate_limit(agent: AgentPubKey) -> ExternResult<bool> {
    let now = sys_time()?;
    let window_start = now - Duration::from_secs(60);

    // Query recent calls by this agent
    let recent_calls = query_recent_calls(agent, window_start)?;

    if recent_calls.len() >= MAX_CALLS_PER_MINUTE as usize {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Rate limit exceeded".into()
        )));
    }

    Ok(true)
}
```

**API gateway rate limiting:**

```yaml
# Kong rate limiting plugin
plugins:
  - name: rate-limiting
    config:
      minute: 100
      hour: 1000
      policy: local
      fault_tolerant: true
      hide_client_headers: false
```

**EVM transaction rate limiting:**

Use nonce management and transaction queuing:

```typescript
import { NonceManager } from '@ethersproject/experimental';

const managedSigner = new NonceManager(signer);

// Queue transactions with rate limiting
const queue = new PQueue({ concurrency: 1, interval: 1000, intervalCap: 5 });

await queue.add(() => contract.someFunction());
```

---

## Appendix: Quick Reference

### Environment Variables

```bash
# Holochain
HOLOCHAIN_ADMIN_URL=ws://localhost:4444
HOLOCHAIN_APP_URL=ws://localhost:8888
LAIR_SOCKET_PATH=/var/run/lair-keystore/socket

# EVM
RPC_URL=https://mainnet.infura.io/v3/<project_id>
PRIVATE_KEY=0x...  # Only for development
ETHERSCAN_API_KEY=<key>

# Contract Addresses
WOUND_ESCROW_ADDRESS=0x...
KENOSIS_BURN_ADDRESS=0x...
FRACTAL_DAO_ADDRESS=0x...
FLOW_TOKEN_ADDRESS=0x...
REPUTATION_TOKEN_ADDRESS=0x...
```

### CLI Commands Cheat Sheet

```bash
# Build
cargo build --release --target wasm32-unknown-unknown
forge build

# Test
cargo test
forge test

# Package
hc dna pack dna/ -o dna/mycelix-living-protocol.dna
hc app pack . -o mycelix-living-protocol.happ

# Deploy
hc sandbox create -n 2 -d mycelix-test
forge create contracts/WoundEscrow.sol:WoundEscrow --constructor-args $FLOW_TOKEN --rpc-url $RPC_URL --private-key $KEY

# Verify
forge verify-contract <address> contracts/WoundEscrow.sol:WoundEscrow --etherscan-api-key $KEY
```

### Support

- Documentation: https://docs.mycelix.io
- GitHub Issues: https://github.com/mycelix/mycelix-v6-living/issues
- Discord: https://discord.gg/mycelix
