---
sidebar_position: 2
title: Installation
---

# Installation

Get Mycelix running on your system in under 2 minutes.

## Prerequisites

Ensure you have one of the following runtimes:

| Runtime | Minimum Version | Recommended |
|---------|-----------------|-------------|
| Node.js | 18.0 | 20.x LTS |
| Bun | 1.0 | 1.1+ |
| Deno | 1.40 | Latest |

## Package Installation

### Using npm

```bash
npm install @mycelix/core @mycelix/server
```

### Using pnpm

```bash
pnpm add @mycelix/core @mycelix/server
```

### Using Bun

```bash
bun add @mycelix/core @mycelix/server
```

## SDK Installation

Choose your preferred language SDK:

### TypeScript/JavaScript

```bash
npm install @mycelix/sdk
```

### Python

```bash
pip install mycelix
# or with uv
uv add mycelix
```

### Go

```bash
go get github.com/mycelix/mycelix-go
```

## Docker Installation

For containerized deployments:

```bash
docker pull mycelix/server:latest
docker run -p 8080:8080 -p 9090:9090 mycelix/server:latest
```

### Docker Compose

```yaml
version: '3.8'
services:
  mycelix:
    image: mycelix/server:latest
    ports:
      - "8080:8080"   # HTTP/REST
      - "9090:9090"   # WebSocket
    volumes:
      - ./config:/etc/mycelix
      - mycelix-data:/var/lib/mycelix
    environment:
      - MYCELIX_CYCLE_START=2024-01-01
      - MYCELIX_LOG_LEVEL=info

volumes:
  mycelix-data:
```

## Verify Installation

Create a test file to verify everything works:

```typescript
// verify.ts
import { Mycelix } from '@mycelix/core';

const mycelix = new Mycelix();
console.log('Mycelix version:', mycelix.version);
console.log('Current phase:', mycelix.currentPhase);
console.log('Cycle day:', mycelix.cycleDay);
```

Run it:

```bash
npx tsx verify.ts
# or
bun verify.ts
```

Expected output:

```
Mycelix version: 1.0.0
Current phase: Dawn
Cycle day: 3
```

## Configuration File

Create `mycelix.config.ts` in your project root:

```typescript
import { defineConfig } from '@mycelix/core';

export default defineConfig({
  // Cycle configuration
  cycle: {
    startDate: '2024-01-01',
    timezone: 'UTC',
  },

  // Server settings
  server: {
    http: { port: 8080 },
    ws: { port: 9090 },
  },

  // Enable primitives
  primitives: {
    enabled: ['pulse', 'spore', 'thread', 'bloom'],
  },
});
```

## Troubleshooting

### Port Already in Use

```bash
# Find what's using the port
lsof -i :8080
# Kill the process or use a different port
MYCELIX_HTTP_PORT=3000 mycelix start
```

### Permission Denied

```bash
# On Linux/macOS, ensure proper permissions
chmod +x ./node_modules/.bin/mycelix
```

### Node Version Too Old

```bash
# Check your Node version
node --version
# Use nvm to install a newer version
nvm install 20
nvm use 20
```

## Next Steps

- [Quick Start](./quick-start) - Build your first Mycelix application
- [Configuration Reference](./server/configuration) - All configuration options
- [Deployment Guide](./server/deployment) - Production deployment strategies
