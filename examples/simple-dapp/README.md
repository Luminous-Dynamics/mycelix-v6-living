# Mycelix Living Protocol Example dApp

This example demonstrates how to integrate with the Mycelix v6.0 Living Protocol
using the TypeScript SDK.

## Prerequisites

- Node.js 18+
- A running Holochain conductor with the Living Protocol DNA installed

## Installation

```bash
npm install
```

## Running

```bash
# Build TypeScript
npm run build

# Run the demo
npm start

# Or run in development mode with auto-reload
npm run dev
```

## Configuration

Set environment variables to customize the connection:

```bash
export HOLOCHAIN_URL=ws://localhost:8888
export APP_ID=mycelix-living-protocol
npm start
```

## What This Demo Shows

1. **Cycle Information** - Displays the current 28-day metabolism cycle state
2. **Wound Healing** - Creates a wound and advances through all healing phases
3. **Composting** - Starts composting a failed proposal
4. **K-Vector** - Submits an 8-dimensional consciousness snapshot

## Code Structure

- `src/index.ts` - Main demo application
- Demonstrates:
  - Connecting to Holochain
  - Calling zome functions
  - Using SDK type definitions
  - Handling the 28-day cycle

## Learn More

- [Living Protocol README](../../README.md)
- [Integration Guide](../../INTEGRATION.md)
- [Architecture Docs](../../docs/ARCHITECTURE.md)
