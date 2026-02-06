---
sidebar_position: 2
title: Deployment Guide
---

# Deployment Guide

Deploy Mycelix to production with confidence.

## Deployment Options

| Option | Best For | Complexity |
|--------|----------|------------|
| Docker | Single node, development | Low |
| Docker Compose | Small clusters | Low |
| Kubernetes | Production, auto-scaling | Medium |
| Bare Metal | Maximum control | High |

## Docker Deployment

### Single Node

```bash
docker run -d \
  --name mycelix \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config:/etc/mycelix \
  -v mycelix-data:/var/lib/mycelix \
  -e MYCELIX_CYCLE_START=2024-01-01 \
  mycelix/server:latest
```

### Custom Configuration

```dockerfile
# Dockerfile
FROM mycelix/server:latest

COPY mycelix.config.ts /etc/mycelix/
COPY primitives/ /app/primitives/

ENV MYCELIX_LOG_LEVEL=info
ENV NODE_ENV=production
```

```bash
docker build -t my-mycelix .
docker run -d -p 8080:8080 -p 9090:9090 my-mycelix
```

## Docker Compose

### Development Cluster

```yaml
# docker-compose.yml
version: '3.8'

services:
  mycelix-1:
    image: mycelix/server:latest
    ports:
      - "8080:8080"
      - "9090:9090"
    environment:
      - MYCELIX_NODE_ID=node-1
      - MYCELIX_CLUSTER_NODES=mycelix-1:7946,mycelix-2:7946,mycelix-3:7946
      - MYCELIX_CYCLE_START=2024-01-01
    volumes:
      - ./config:/etc/mycelix
      - node1-data:/var/lib/mycelix

  mycelix-2:
    image: mycelix/server:latest
    ports:
      - "8081:8080"
      - "9091:9090"
    environment:
      - MYCELIX_NODE_ID=node-2
      - MYCELIX_CLUSTER_NODES=mycelix-1:7946,mycelix-2:7946,mycelix-3:7946
    volumes:
      - ./config:/etc/mycelix
      - node2-data:/var/lib/mycelix

  mycelix-3:
    image: mycelix/server:latest
    ports:
      - "8082:8080"
      - "9092:9090"
    environment:
      - MYCELIX_NODE_ID=node-3
      - MYCELIX_CLUSTER_NODES=mycelix-1:7946,mycelix-2:7946,mycelix-3:7946
    volumes:
      - ./config:/etc/mycelix
      - node3-data:/var/lib/mycelix

volumes:
  node1-data:
  node2-data:
  node3-data:
```

### Production Compose

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  mycelix:
    image: mycelix/server:latest
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 4G
        reservations:
          cpus: '1'
          memory: 2G
      restart_policy:
        condition: on-failure
        delay: 5s
        max_attempts: 3
    ports:
      - "8080:8080"
      - "9090:9090"
    environment:
      - MYCELIX_CLUSTER_NODES=tasks.mycelix:7946
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    volumes:
      - /etc/mycelix:/etc/mycelix:ro
    secrets:
      - tls_cert
      - tls_key

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data

  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9091:9090"

secrets:
  tls_cert:
    file: ./certs/server.crt
  tls_key:
    file: ./certs/server.key

volumes:
  redis-data:
```

## Kubernetes Deployment

### Namespace and ConfigMap

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: mycelix

---
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: mycelix-config
  namespace: mycelix
data:
  mycelix.config.json: |
    {
      "cycle": {
        "startDate": "2024-01-01",
        "timezone": "UTC"
      },
      "server": {
        "http": { "port": 8080 },
        "ws": { "port": 9090 }
      },
      "cluster": {
        "discovery": {
          "method": "kubernetes",
          "kubernetes": {
            "namespace": "mycelix",
            "labelSelector": "app=mycelix"
          }
        }
      }
    }
```

### StatefulSet

```yaml
# statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: mycelix
  namespace: mycelix
spec:
  serviceName: mycelix
  replicas: 3
  selector:
    matchLabels:
      app: mycelix
  template:
    metadata:
      labels:
        app: mycelix
    spec:
      containers:
        - name: mycelix
          image: mycelix/server:latest
          ports:
            - containerPort: 8080
              name: http
            - containerPort: 9090
              name: ws
            - containerPort: 7946
              name: gossip
          env:
            - name: MYCELIX_NODE_ID
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
          volumeMounts:
            - name: config
              mountPath: /etc/mycelix
            - name: data
              mountPath: /var/lib/mycelix
          resources:
            requests:
              cpu: "500m"
              memory: "1Gi"
            limits:
              cpu: "2"
              memory: "4Gi"
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 30
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /ready
              port: 8080
            initialDelaySeconds: 5
            periodSeconds: 5
      volumes:
        - name: config
          configMap:
            name: mycelix-config
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes: ["ReadWriteOnce"]
        resources:
          requests:
            storage: 10Gi
```

### Services

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: mycelix
  namespace: mycelix
spec:
  selector:
    app: mycelix
  ports:
    - name: http
      port: 8080
      targetPort: 8080
    - name: ws
      port: 9090
      targetPort: 9090
  clusterIP: None  # Headless for StatefulSet

---
apiVersion: v1
kind: Service
metadata:
  name: mycelix-lb
  namespace: mycelix
spec:
  type: LoadBalancer
  selector:
    app: mycelix
  ports:
    - name: http
      port: 80
      targetPort: 8080
    - name: ws
      port: 443
      targetPort: 9090
```

### Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mycelix
  namespace: mycelix
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: StatefulSet
    name: mycelix
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
        - type: Pods
          value: 2
          periodSeconds: 60
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
        - type: Pods
          value: 1
          periodSeconds: 120
```

### Helm Chart

```bash
# Install via Helm
helm repo add mycelix https://charts.mycelix.io
helm install mycelix mycelix/mycelix \
  --namespace mycelix \
  --create-namespace \
  --set replicas=3 \
  --set cycle.startDate=2024-01-01
```

## Cloud-Specific Deployments

### AWS ECS

```json
{
  "family": "mycelix",
  "containerDefinitions": [
    {
      "name": "mycelix",
      "image": "mycelix/server:latest",
      "cpu": 1024,
      "memory": 2048,
      "portMappings": [
        { "containerPort": 8080, "protocol": "tcp" },
        { "containerPort": 9090, "protocol": "tcp" }
      ],
      "environment": [
        { "name": "MYCELIX_CYCLE_START", "value": "2024-01-01" }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/mycelix",
          "awslogs-region": "us-east-1"
        }
      }
    }
  ]
}
```

### Google Cloud Run

```yaml
# service.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: mycelix
spec:
  template:
    spec:
      containers:
        - image: gcr.io/project/mycelix:latest
          ports:
            - containerPort: 8080
          env:
            - name: MYCELIX_CYCLE_START
              value: "2024-01-01"
          resources:
            limits:
              cpu: "2"
              memory: "2Gi"
```

## Health Checks

Configure health endpoints:

```typescript
// Health check configuration
export default defineConfig({
  health: {
    // Liveness - is the process alive?
    liveness: {
      path: '/health',
      checks: ['process'],
    },

    // Readiness - can it receive traffic?
    readiness: {
      path: '/ready',
      checks: ['process', 'storage', 'cluster'],
    },

    // Startup - has it finished initializing?
    startup: {
      path: '/startup',
      timeout: '60s',
    },
  },
});
```

## Phase-Aware Deployment

Consider the cycle when deploying:

| Phase | Deployment Recommendation |
|-------|---------------------------|
| Dawn | Ideal for major releases |
| Surge | Avoid deployments if possible |
| Settle | Safe for minor updates |
| Rest | Best for maintenance |

```bash
# Check current phase before deploying
curl -s http://mycelix:8080/cycle | jq '.phase'

# Conditional deployment script
PHASE=$(curl -s http://mycelix:8080/cycle | jq -r '.phase')
if [ "$PHASE" = "Dawn" ] || [ "$PHASE" = "Rest" ]; then
  kubectl rollout restart deployment/mycelix
else
  echo "Current phase ($PHASE) not ideal for deployment"
  exit 1
fi
```

## Next Steps

- [Security Configuration](./security) - Secure your deployment
- [Configuration Reference](./configuration) - All configuration options
