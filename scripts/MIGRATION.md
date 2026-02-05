# Mycelix v5.2 to v6.0 Migration Guide

This guide describes how to migrate from Mycelix v5.2 (mycelix-property) to
v6.0 (mycelix-living-protocol).

## Overview

The migration involves:
1. Running both DNAs in parallel during transition
2. Converting existing data to new formats
3. Gradually enabling v6.0 features
4. Disabling v5.2 punitive features once v6.0 is validated

## Prerequisites

- Both v5.2 and v6.0 Holochain conductors running
- Node.js 18+ for migration scripts
- Backup of v5.2 data

## Migration Steps

### Step 1: Deploy v6.0 DNA

Deploy the v6.0 DNA alongside v5.2:

```bash
# Build v6.0 zomes
./scripts/mycelix-cli.sh zome build

# Pack and install DNA
hc dna pack dna/
hc app pack .
hc sandbox install mycelix-living-protocol.happ
```

### Step 2: Enable Bridge Zome

The bridge zome enables cross-DNA communication:

```bash
# Verify bridge is working
curl -X POST http://localhost:8888/api/v1/call \
  -d '{"zome": "bridge", "fn": "get_migration_status", "payload": null}'
```

### Step 3: Run Migration Script

```bash
# Set environment variables
export V52_URL=ws://localhost:8888
export V60_URL=ws://localhost:8889

# Run migration
npx tsx scripts/migrate-v52-to-v60.ts
```

### Step 4: Verify Migration

Check migration results:

```bash
# Check MATL -> MetabolicTrust
curl -X POST http://localhost:8889/api/v1/call \
  -d '{"zome": "living_metabolism", "fn": "get_metabolic_trust_count", "payload": null}'

# Check Slash -> Wound conversion
curl -X POST http://localhost:8889/api/v1/call \
  -d '{"zome": "living_metabolism", "fn": "get_wound_count", "payload": null}'
```

### Step 5: Enable Feature Flags

Gradually enable v6.0 features:

```rust
// In configuration
let config = LivingProtocolConfig {
    features: FeatureFlags {
        slash_interception: true,  // Convert slashes to wounds
        beauty_scoring: true,      // Enable beauty evaluation
        phi_measurement: false,    // Keep disabled initially
        ..Default::default()
    },
    ..Default::default()
};
```

### Step 6: Monitoring Period

Run both systems in parallel for a cycle (28 days):

- Monitor wound healing outcomes
- Compare with v5.2 slashing results
- Gather community feedback
- Adjust configuration as needed

### Step 7: Disable v5.2 Slashing

Once confident, disable v5.2 punitive slashing:

```bash
# In v5.2 configuration
ENABLE_SLASHING=false
```

## Data Mapping

### MATL Score → MetabolicTrust

| v5.2 Field | v6.0 Field |
|------------|------------|
| `score` | `trust_score` |
| `throughput` | `throughput_component` |
| `resilience` | `resilience_component` |
| - | `composting_contribution` (new) |

### Slash → Wound

| v5.2 Slash % | v6.0 Severity | Healing Days |
|-------------|--------------|--------------|
| 1-5% | Minor | 3 |
| 5-15% | Moderate | 7 |
| 15-30% | Severe | 14 |
| 30%+ | Critical | 28 |

### K-Vector (5D → 8D)

| v5.2 Dimension | v6.0 Dimension |
|----------------|----------------|
| stability | presence |
| adaptability | coherence |
| integrity | receptivity |
| connectivity | integration |
| emergence | generativity |
| - | surrender (new, default 0.5) |
| - | discernment (new, default 0.5) |
| - | emergence (new, default 0.5) |

## Rollback Procedure

If issues are detected:

1. Disable slash interception: `slash_interception: false`
2. Re-enable v5.2 slashing: `ENABLE_SLASHING=true`
3. Report issues to the team
4. Analyze wound healing data

## Troubleshooting

### Connection Errors

```
Error: Failed to connect to v5.2 conductor
```

Solution: Ensure both conductors are running and accessible.

### Missing Data

```
Error: No MATL scores found
```

Solution: Verify v5.2 DNA has data. Check agent permissions.

### Conversion Errors

```
Error: Invalid wound severity
```

Solution: Check slash percentage range. May need manual review.

## Support

- [Integration Guide](../INTEGRATION.md)
- [Architecture Docs](../docs/ARCHITECTURE.md)
- [GitHub Issues](https://github.com/mycelix/mycelix-v6-living/issues)
