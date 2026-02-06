# TLS Configuration for Mycelix WebSocket Server

The Mycelix WebSocket server does not handle TLS directly. Instead, we recommend using a reverse proxy for TLS termination. This approach provides:

- Centralized certificate management
- Automatic certificate renewal (with Let's Encrypt)
- Better security through separation of concerns
- Flexibility to add load balancing and other features

## Overview

```
                    +-----------------+
Client --[TLS]--> | Reverse Proxy   | --[HTTP/WS]--> Mycelix Server
  wss://          | (nginx/traefik) |                :8888
                    +-----------------+
```

## Nginx Configuration

### Basic Setup with Let's Encrypt

1. Install nginx and certbot:

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install nginx certbot python3-certbot-nginx

# RHEL/CentOS
sudo dnf install nginx certbot python3-certbot-nginx
```

2. Create nginx configuration at `/etc/nginx/sites-available/mycelix`:

```nginx
upstream mycelix_ws {
    server 127.0.0.1:8888;
    keepalive 32;
}

upstream mycelix_health {
    server 127.0.0.1:8889;
}

server {
    listen 80;
    server_name mycelix.example.com;

    # Redirect HTTP to HTTPS
    location / {
        return 301 https://$server_name$request_uri;
    }

    # Let's Encrypt challenge
    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }
}

server {
    listen 443 ssl http2;
    server_name mycelix.example.com;

    # SSL certificates (managed by certbot)
    ssl_certificate /etc/letsencrypt/live/mycelix.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/mycelix.example.com/privkey.pem;

    # SSL configuration
    ssl_session_timeout 1d;
    ssl_session_cache shared:SSL:50m;
    ssl_session_tickets off;

    # Modern TLS configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # HSTS
    add_header Strict-Transport-Security "max-age=63072000" always;

    # WebSocket endpoint
    location / {
        proxy_pass http://mycelix_ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Pass API key header if present
        proxy_set_header X-API-Key $http_x_api_key;

        # WebSocket timeouts
        proxy_read_timeout 86400;
        proxy_send_timeout 86400;
        proxy_connect_timeout 60;
    }

    # Health check endpoint (optional public exposure)
    location /health {
        proxy_pass http://mycelix_health/health;
        proxy_set_header Host $host;
    }

    # Metrics endpoint (restrict access)
    location /metrics {
        # Restrict to internal networks
        allow 10.0.0.0/8;
        allow 172.16.0.0/12;
        allow 192.168.0.0/16;
        allow 127.0.0.1;
        deny all;

        proxy_pass http://mycelix_health/metrics;
        proxy_set_header Host $host;
    }
}
```

3. Enable the site and obtain certificate:

```bash
sudo ln -s /etc/nginx/sites-available/mycelix /etc/nginx/sites-enabled/
sudo nginx -t
sudo certbot --nginx -d mycelix.example.com
sudo systemctl reload nginx
```

### Rate Limiting at Nginx Level

You can add an additional layer of rate limiting at the nginx level:

```nginx
# In http block
limit_req_zone $binary_remote_addr zone=mycelix_limit:10m rate=100r/s;
limit_conn_zone $binary_remote_addr zone=mycelix_conn:10m;

# In server block
location / {
    limit_req zone=mycelix_limit burst=200 nodelay;
    limit_conn mycelix_conn 10;

    # ... rest of proxy configuration
}
```

## Traefik Configuration

### Docker Compose Setup

```yaml
version: '3.8'

services:
  traefik:
    image: traefik:v3.0
    command:
      - "--api.dashboard=true"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.httpchallenge.entrypoint=web"
      - "--certificatesresolvers.letsencrypt.acme.email=admin@example.com"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - "/var/run/docker.sock:/var/run/docker.sock:ro"
      - "letsencrypt:/letsencrypt"
    networks:
      - traefik

  mycelix:
    image: mycelix/ws-server:latest
    command:
      - "--host=0.0.0.0"
      - "--port=8888"
      - "--require-auth"
      - "--api-keys=${MYCELIX_API_KEYS}"
      - "--max-connections=10000"
      - "--rate-limit=100"
    labels:
      - "traefik.enable=true"
      # HTTP to HTTPS redirect
      - "traefik.http.routers.mycelix-http.rule=Host(`mycelix.example.com`)"
      - "traefik.http.routers.mycelix-http.entrypoints=web"
      - "traefik.http.routers.mycelix-http.middlewares=https-redirect"
      - "traefik.http.middlewares.https-redirect.redirectscheme.scheme=https"
      # HTTPS with TLS
      - "traefik.http.routers.mycelix.rule=Host(`mycelix.example.com`)"
      - "traefik.http.routers.mycelix.entrypoints=websecure"
      - "traefik.http.routers.mycelix.tls=true"
      - "traefik.http.routers.mycelix.tls.certresolver=letsencrypt"
      - "traefik.http.services.mycelix.loadbalancer.server.port=8888"
      # Rate limiting middleware
      - "traefik.http.middlewares.mycelix-ratelimit.ratelimit.average=100"
      - "traefik.http.middlewares.mycelix-ratelimit.ratelimit.burst=200"
      - "traefik.http.routers.mycelix.middlewares=mycelix-ratelimit"
    networks:
      - traefik
    expose:
      - "8888"
      - "8889"

volumes:
  letsencrypt:

networks:
  traefik:
    external: true
```

### Traefik Static Configuration (traefik.yml)

```yaml
entryPoints:
  web:
    address: ":80"
    http:
      redirections:
        entryPoint:
          to: websecure
          scheme: https

  websecure:
    address: ":443"
    http:
      tls:
        certResolver: letsencrypt

certificatesResolvers:
  letsencrypt:
    acme:
      email: admin@example.com
      storage: /letsencrypt/acme.json
      httpChallenge:
        entryPoint: web

providers:
  docker:
    exposedByDefault: false
    network: traefik
```

## Kubernetes Ingress

### Using nginx-ingress

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: mycelix-ingress
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/proxy-read-timeout: "86400"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "86400"
    nginx.ingress.kubernetes.io/upstream-hash-by: "$remote_addr"
    nginx.ingress.kubernetes.io/limit-rps: "100"
    nginx.ingress.kubernetes.io/limit-connections: "10"
spec:
  tls:
    - hosts:
        - mycelix.example.com
      secretName: mycelix-tls
  rules:
    - host: mycelix.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: mycelix-ws-server
                port:
                  number: 8888
```

### Using Traefik IngressRoute

```yaml
apiVersion: traefik.io/v1alpha1
kind: IngressRoute
metadata:
  name: mycelix-websecure
spec:
  entryPoints:
    - websecure
  routes:
    - match: Host(`mycelix.example.com`)
      kind: Rule
      services:
        - name: mycelix-ws-server
          port: 8888
      middlewares:
        - name: mycelix-ratelimit
  tls:
    certResolver: letsencrypt

---
apiVersion: traefik.io/v1alpha1
kind: Middleware
metadata:
  name: mycelix-ratelimit
spec:
  rateLimit:
    average: 100
    burst: 200
```

## Self-Signed Certificates (Development Only)

For development environments, you can generate self-signed certificates:

```bash
# Generate CA
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 1826 -key ca.key -out ca.crt \
    -subj "/CN=Mycelix Development CA"

# Generate server certificate
openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr \
    -subj "/CN=localhost"
openssl x509 -req -days 365 -in server.csr -CA ca.crt -CAkey ca.key \
    -CAcreateserial -out server.crt \
    -extfile <(printf "subjectAltName=DNS:localhost,IP:127.0.0.1")

# Use with nginx
# ssl_certificate /path/to/server.crt;
# ssl_certificate_key /path/to/server.key;
```

## Client Configuration

When connecting to a TLS-enabled server:

### TypeScript/JavaScript

```typescript
import { MycelixClient } from '@mycelix/sdk';

const client = new MycelixClient({
  url: 'wss://mycelix.example.com',  // Note: wss:// for TLS
  apiKey: 'your-api-key',
});

await client.connect();
```

### Command Line (websocat)

```bash
# With trusted certificate
websocat wss://mycelix.example.com

# With API key header
websocat -H "X-API-Key: your-api-key" wss://mycelix.example.com

# Self-signed certificate (development)
websocat --insecure wss://localhost:8443
```

## Security Best Practices

1. **Always use TLS in production** - Never expose the WebSocket server directly without TLS.

2. **Use modern TLS versions** - Only enable TLS 1.2 and 1.3.

3. **Enable HSTS** - Prevent protocol downgrade attacks.

4. **Restrict metrics endpoint** - Only allow internal access to `/metrics`.

5. **Use API keys** - Enable `--require-auth` in production.

6. **Rate limit at multiple levels** - Configure rate limiting in both the proxy and the server.

7. **Monitor certificate expiry** - Set up alerts for certificate renewal failures.

8. **Regular security updates** - Keep nginx/traefik and dependencies updated.

## Troubleshooting

### WebSocket connection fails

1. Check nginx logs: `sudo tail -f /var/log/nginx/error.log`
2. Verify WebSocket upgrade headers are being passed
3. Ensure `proxy_read_timeout` is high enough for long-lived connections

### Certificate issues

1. Test certificate: `openssl s_client -connect mycelix.example.com:443`
2. Check certbot logs: `sudo certbot certificates`
3. Verify certificate chain is complete

### Connection drops

1. Increase `proxy_read_timeout` and `proxy_send_timeout`
2. Check for firewall rules blocking long-lived connections
3. Verify load balancer health checks aren't interfering
