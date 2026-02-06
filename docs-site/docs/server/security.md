---
sidebar_position: 3
title: Security Configuration
---

# Security Configuration

Secure your Mycelix deployment with authentication, authorization, and encryption.

## Security Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Security Layers                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   TLS/mTLS  │  │   AuthN     │  │      AuthZ          │  │
│  │  Transport  │→ │  Identity   │→ │   Permissions       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│         │               │                    │              │
│         ▼               ▼                    ▼              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Rate Limiting & Audit Logging              ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## Transport Security (TLS)

### Basic TLS

```typescript
export default defineConfig({
  tls: {
    enabled: true,
    cert: '/etc/mycelix/tls/server.crt',
    key: '/etc/mycelix/tls/server.key',

    // Minimum TLS version
    minVersion: 'TLSv1.2',

    // Cipher suites
    ciphers: [
      'TLS_AES_256_GCM_SHA384',
      'TLS_CHACHA20_POLY1305_SHA256',
      'TLS_AES_128_GCM_SHA256',
    ],
  },
});
```

### Mutual TLS (mTLS)

```typescript
export default defineConfig({
  tls: {
    enabled: true,
    cert: '/etc/mycelix/tls/server.crt',
    key: '/etc/mycelix/tls/server.key',

    // Require client certificates
    clientAuth: 'require', // 'none' | 'request' | 'require'
    clientCa: '/etc/mycelix/tls/client-ca.crt',

    // Verify client certificate
    verifyClient: true,
    verifyDepth: 2,
  },
});
```

### Certificate Generation

```bash
# Generate CA
openssl genrsa -out ca.key 4096
openssl req -x509 -new -nodes -key ca.key -sha256 -days 365 \
  -out ca.crt -subj "/CN=Mycelix CA"

# Generate server certificate
openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr \
  -subj "/CN=mycelix.example.com"
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key \
  -CAcreateserial -out server.crt -days 365 -sha256

# Generate client certificate
openssl genrsa -out client.key 2048
openssl req -new -key client.key -out client.csr \
  -subj "/CN=mycelix-client"
openssl x509 -req -in client.csr -CA ca.crt -CAkey ca.key \
  -CAcreateserial -out client.crt -days 365 -sha256
```

## Authentication

### API Key Authentication

```typescript
export default defineConfig({
  auth: {
    method: 'apikey',
    apikey: {
      header: 'X-API-Key',
      // Keys can be static or from external source
      keys: [
        { key: 'mk_prod_xxx', name: 'production', roles: ['admin'] },
        { key: 'mk_dev_xxx', name: 'development', roles: ['read'] },
      ],
      // Or load from file/env
      keysFile: '/etc/mycelix/api-keys.json',
    },
  },
});
```

### JWT Authentication

```typescript
export default defineConfig({
  auth: {
    method: 'jwt',
    jwt: {
      // JWT verification
      secret: process.env.JWT_SECRET,
      // Or use public key for RS256
      publicKey: '/etc/mycelix/jwt-public.pem',
      algorithms: ['RS256', 'ES256'],

      // Token location
      header: 'Authorization',
      scheme: 'Bearer',

      // Claims mapping
      claims: {
        subject: 'sub',
        roles: 'roles',
        permissions: 'permissions',
      },

      // Issuer validation
      issuer: 'https://auth.example.com',
      audience: 'mycelix-api',
    },
  },
});
```

### OAuth 2.0 / OIDC

```typescript
export default defineConfig({
  auth: {
    method: 'oidc',
    oidc: {
      issuer: 'https://auth.example.com',
      clientId: 'mycelix',
      clientSecret: process.env.OIDC_CLIENT_SECRET,

      // Discovery endpoint
      discoveryUrl: 'https://auth.example.com/.well-known/openid-configuration',

      // Scopes to request
      scopes: ['openid', 'profile', 'mycelix:read', 'mycelix:write'],

      // Token validation
      validateToken: true,
      clockTolerance: 30, // seconds
    },
  },
});
```

## Authorization

### Role-Based Access Control (RBAC)

```typescript
export default defineConfig({
  authz: {
    method: 'rbac',
    rbac: {
      roles: {
        admin: {
          permissions: ['*'],
        },
        operator: {
          permissions: [
            'primitives:read',
            'primitives:write',
            'cycle:read',
            'metrics:read',
          ],
        },
        developer: {
          permissions: [
            'primitives:read',
            'cycle:read',
            'ws:connect',
          ],
        },
        readonly: {
          permissions: [
            'primitives:read',
            'cycle:read',
          ],
        },
      },
    },
  },
});
```

### Permission Definitions

```typescript
export default defineConfig({
  authz: {
    permissions: {
      // Primitive permissions
      'primitives:read': 'Read primitive state',
      'primitives:write': 'Create/update primitives',
      'primitives:delete': 'Delete primitives',

      // Cycle permissions
      'cycle:read': 'Read cycle state',
      'cycle:admin': 'Modify cycle configuration',

      // WebSocket permissions
      'ws:connect': 'Connect via WebSocket',
      'ws:subscribe': 'Subscribe to events',
      'ws:publish': 'Publish messages',

      // Admin permissions
      'admin:config': 'Modify configuration',
      'admin:cluster': 'Manage cluster',
    },
  },
});
```

### Attribute-Based Access Control (ABAC)

```typescript
export default defineConfig({
  authz: {
    method: 'abac',
    abac: {
      policies: [
        {
          name: 'phase-aware-write',
          effect: 'allow',
          condition: {
            // Only allow writes during Dawn or Settle
            'context.phase': { in: ['Dawn', 'Settle'] },
            'action': 'write',
          },
        },
        {
          name: 'department-access',
          effect: 'allow',
          condition: {
            'subject.department': { equals: 'resource.owner' },
          },
        },
      ],
    },
  },
});
```

## Rate Limiting

### Global Rate Limits

```typescript
export default defineConfig({
  rateLimit: {
    enabled: true,

    // Global limits
    global: {
      requests: 10000,
      window: '1m',
    },

    // Per-client limits
    perClient: {
      requests: 100,
      window: '1m',
      keyBy: 'ip', // 'ip' | 'apikey' | 'user'
    },

    // Phase-aware limits
    phaseMultipliers: {
      Dawn: 0.5,
      Surge: 1.0,
      Settle: 0.7,
      Rest: 0.2,
    },
  },
});
```

### Endpoint-Specific Limits

```typescript
export default defineConfig({
  rateLimit: {
    endpoints: {
      '/api/primitives': {
        requests: 1000,
        window: '1m',
      },
      '/ws': {
        connections: 100,
        messagesPerSecond: 50,
      },
      '/graphql': {
        requests: 500,
        window: '1m',
        complexity: 10000, // GraphQL complexity limit
      },
    },
  },
});
```

## Audit Logging

### Configuration

```typescript
export default defineConfig({
  audit: {
    enabled: true,

    // What to log
    events: [
      'auth.login',
      'auth.logout',
      'auth.failed',
      'primitives.create',
      'primitives.update',
      'primitives.delete',
      'config.change',
      'cluster.join',
      'cluster.leave',
    ],

    // Where to log
    output: {
      type: 'file', // 'file' | 'syslog' | 'webhook'
      path: '/var/log/mycelix/audit.log',
      rotation: 'daily',
      retention: '90d',
    },

    // What to include
    include: {
      timestamp: true,
      user: true,
      ip: true,
      action: true,
      resource: true,
      result: true,
      duration: true,
    },
  },
});
```

### Audit Log Format

```json
{
  "timestamp": "2024-03-15T14:30:00.000Z",
  "event": "primitives.create",
  "user": {
    "id": "user-123",
    "name": "alice@example.com",
    "roles": ["developer"]
  },
  "client": {
    "ip": "192.168.1.100",
    "userAgent": "mycelix-sdk/1.0.0"
  },
  "action": {
    "type": "create",
    "resource": "primitives/pulse",
    "name": "heartbeat"
  },
  "result": {
    "success": true,
    "id": "pulse-abc123"
  },
  "context": {
    "phase": "Dawn",
    "cycleDay": 3,
    "node": "node-1"
  },
  "duration": 45
}
```

## Network Security

### IP Allowlisting

```typescript
export default defineConfig({
  network: {
    allowlist: {
      enabled: true,
      ips: [
        '10.0.0.0/8',
        '172.16.0.0/12',
        '192.168.0.0/16',
      ],
      // Bypass for health checks
      bypass: ['/health', '/ready'],
    },
  },
});
```

### CORS Configuration

```typescript
export default defineConfig({
  cors: {
    enabled: true,
    origins: [
      'https://app.example.com',
      'https://admin.example.com',
    ],
    methods: ['GET', 'POST', 'PUT', 'DELETE'],
    headers: ['Authorization', 'Content-Type', 'X-API-Key'],
    credentials: true,
    maxAge: 86400,
  },
});
```

## Secrets Management

### Environment Variables

```bash
# Sensitive values via environment
export MYCELIX_JWT_SECRET="your-secret-key"
export MYCELIX_DB_PASSWORD="db-password"
export MYCELIX_API_KEY_ADMIN="mk_admin_xxx"
```

### Kubernetes Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: mycelix-secrets
  namespace: mycelix
type: Opaque
stringData:
  jwt-secret: "your-secret-key"
  db-password: "db-password"
---
# Reference in deployment
env:
  - name: MYCELIX_JWT_SECRET
    valueFrom:
      secretKeyRef:
        name: mycelix-secrets
        key: jwt-secret
```

### HashiCorp Vault

```typescript
export default defineConfig({
  secrets: {
    provider: 'vault',
    vault: {
      address: 'https://vault.example.com',
      auth: {
        method: 'kubernetes',
        role: 'mycelix',
      },
      path: 'secret/mycelix',
      refresh: '5m',
    },
  },
});
```

## Security Best Practices

### Checklist

- [ ] Enable TLS for all connections
- [ ] Use mTLS for node-to-node communication
- [ ] Implement authentication for all endpoints
- [ ] Apply least-privilege RBAC policies
- [ ] Enable audit logging
- [ ] Configure rate limiting
- [ ] Rotate secrets regularly
- [ ] Keep dependencies updated
- [ ] Run security scans in CI/CD

### Phase-Specific Security

```typescript
export default defineConfig({
  phaseOverrides: {
    // Stricter security during Surge (high traffic)
    Surge: {
      rateLimit: {
        perClient: { requests: 50 },
      },
    },

    // Relaxed limits during Rest (maintenance)
    Rest: {
      auth: {
        // Allow additional admin access during maintenance
        additionalRoles: ['maintenance'],
      },
    },
  },
});
```

## Next Steps

- [Configuration Reference](./configuration) - All configuration options
- [Deployment Guide](./deployment) - Deploy securely to production
