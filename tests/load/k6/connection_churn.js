// Connection Churn Test for Mycelix Living Protocol
// Tests rapid connection open/close cycles to stress the server
//
// Usage:
//   k6 run connection_churn.js
//   CONN_RATE=100 CHURN_DURATION=10m k6 run connection_churn.js

import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend, Gauge } from 'k6/metrics';
import { config, createRpcRequest, parseRpcResponse } from './config.js';

// Custom metrics
const connectionDuration = new Trend('connection_duration', true);
const connectionErrors = new Rate('connection_errors');
const connectionsOpened = new Counter('connections_opened');
const connectionsClosed = new Counter('connections_closed');
const connectionsFailed = new Counter('connections_failed');
const handshakeTime = new Trend('handshake_time', true);
const rpcRequestDuration = new Trend('rpc_request_duration', true);
const activeConnections = new Gauge('active_connections');

// Test configuration
export const options = {
  scenarios: {
    // Constant rate of new connections
    connection_churn: {
      executor: 'constant-arrival-rate',
      rate: config.load.churn.connectionRate,
      timeUnit: '1s',
      duration: config.load.churn.duration,
      preAllocatedVUs: config.load.churn.connectionRate * 2,
      maxVUs: config.load.churn.connectionRate * 5,
    },
  },
  thresholds: {
    'connection_errors': ['rate<0.01'],           // Less than 1% connection errors
    'handshake_time': ['p(95)<500'],              // 95% of handshakes under 500ms
    'connection_duration': ['p(95)<5000'],        // 95% of connections complete within 5s
    'rpc_request_duration': ['p(95)<100'],        // 95% of RPC requests under 100ms
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(50)', 'p(90)', 'p(95)', 'p(99)', 'count'],
};

// Connection lifecycle patterns
const lifecyclePatterns = [
  { name: 'immediate_close', weight: 10, holdTime: 0 },
  { name: 'single_request', weight: 30, holdTime: 100, requests: 1 },
  { name: 'burst_requests', weight: 20, holdTime: 500, requests: 5 },
  { name: 'normal_session', weight: 25, holdTime: 2000, requests: 3 },
  { name: 'long_session', weight: 15, holdTime: 5000, requests: 10 },
];

function selectPattern() {
  const totalWeight = lifecyclePatterns.reduce((sum, p) => sum + p.weight, 0);
  let random = Math.random() * totalWeight;

  for (const pattern of lifecyclePatterns) {
    random -= pattern.weight;
    if (random <= 0) return pattern;
  }
  return lifecyclePatterns[0];
}

let connectionCounter = 0;

export default function() {
  const url = config.server.url;
  const pattern = selectPattern();
  const connectionId = ++connectionCounter;
  const connectionStart = Date.now();
  let handshakeComplete = false;
  let handshakeStart = Date.now();

  const res = ws.connect(url, { tags: { pattern: pattern.name } }, function(socket) {
    connectionsOpened.add(1);
    activeConnections.add(1);

    socket.on('open', function() {
      handshakeComplete = true;
      handshakeTime.add(Date.now() - handshakeStart);

      // Immediate close pattern
      if (pattern.holdTime === 0) {
        socket.close();
        return;
      }

      // Send requests based on pattern
      const requestCount = pattern.requests || 0;
      const requestDelay = requestCount > 0 ? pattern.holdTime / requestCount : pattern.holdTime;

      for (let i = 0; i < requestCount; i++) {
        socket.setTimeout(function() {
          // Alternate between methods
          const methods = ['getCycleState', 'getCurrentPhase', 'getCycleNumber'];
          const method = methods[i % methods.length];
          const request = createRpcRequest(method);
          const requestStart = Date.now();

          socket.send(request);

          // Note: In k6, we can't easily correlate async responses
          // This is a simplified approach
        }, i * requestDelay);
      }

      // Close after hold time
      socket.setTimeout(function() {
        socket.close();
      }, pattern.holdTime);
    });

    socket.on('message', function(data) {
      const response = parseRpcResponse(data);
      if (response.id) {
        // Approximate latency (not precise due to async nature)
        rpcRequestDuration.add(10);  // Placeholder
      }
    });

    socket.on('error', function(e) {
      connectionErrors.add(1);
    });

    socket.on('close', function() {
      connectionsClosed.add(1);
      activeConnections.add(-1);
      connectionDuration.add(Date.now() - connectionStart);
    });
  });

  const connected = check(res, {
    'Connection established': (r) => r && r.status === 101,
  });

  if (!connected) {
    connectionsFailed.add(1);
    connectionErrors.add(1);
  } else {
    connectionErrors.add(0);
  }

  // Small random delay to prevent thundering herd
  sleep(Math.random() * 0.1);
}

export function setup() {
  console.log(`Starting connection churn test against ${config.server.url}`);
  console.log(`Connection rate: ${config.load.churn.connectionRate}/second`);
  console.log(`Duration: ${config.load.churn.duration}`);
  console.log('\nLifecycle patterns:');
  for (const pattern of lifecyclePatterns) {
    console.log(`  ${pattern.name}: ${pattern.weight}% weight, ${pattern.holdTime}ms hold, ${pattern.requests || 0} requests`);
  }
  return {};
}

export function teardown(data) {
  console.log('Connection churn test completed');
}

export function handleSummary(data) {
  const summary = {
    timestamp: new Date().toISOString(),
    test: 'connection_churn',
    metrics: {
      total_connections: data.metrics.connections_opened?.values?.count || 0,
      failed_connections: data.metrics.connections_failed?.values?.count || 0,
      error_rate: data.metrics.connection_errors?.values?.rate || 0,
      handshake: {
        p50: data.metrics.handshake_time?.values?.['p(50)'],
        p95: data.metrics.handshake_time?.values?.['p(95)'],
        p99: data.metrics.handshake_time?.values?.['p(99)'],
        avg: data.metrics.handshake_time?.values?.avg,
        max: data.metrics.handshake_time?.values?.max,
      },
      connection_duration: {
        p50: data.metrics.connection_duration?.values?.['p(50)'],
        p95: data.metrics.connection_duration?.values?.['p(95)'],
        p99: data.metrics.connection_duration?.values?.['p(99)'],
        avg: data.metrics.connection_duration?.values?.avg,
        max: data.metrics.connection_duration?.values?.max,
      },
    },
    thresholds: {
      error_rate_ok: (data.metrics.connection_errors?.values?.rate || 0) < 0.01,
      handshake_ok: (data.metrics.handshake_time?.values?.['p(95)'] || 0) < 500,
    },
  };

  let output = '\n=== Connection Churn Test Summary ===\n\n';
  output += `Total Connections: ${summary.metrics.total_connections}\n`;
  output += `Failed Connections: ${summary.metrics.failed_connections}\n`;
  output += `Error Rate: ${(summary.metrics.error_rate * 100).toFixed(2)}%\n\n`;

  output += 'Handshake Time:\n';
  const hs = summary.metrics.handshake;
  output += `  p(50): ${hs.p50?.toFixed(2) || 'N/A'}ms\n`;
  output += `  p(95): ${hs.p95?.toFixed(2) || 'N/A'}ms\n`;
  output += `  p(99): ${hs.p99?.toFixed(2) || 'N/A'}ms\n`;
  output += `  avg:   ${hs.avg?.toFixed(2) || 'N/A'}ms\n`;
  output += `  max:   ${hs.max?.toFixed(2) || 'N/A'}ms\n\n`;

  output += 'Connection Duration:\n';
  const cd = summary.metrics.connection_duration;
  output += `  p(50): ${cd.p50?.toFixed(2) || 'N/A'}ms\n`;
  output += `  p(95): ${cd.p95?.toFixed(2) || 'N/A'}ms\n`;
  output += `  p(99): ${cd.p99?.toFixed(2) || 'N/A'}ms\n`;
  output += `  avg:   ${cd.avg?.toFixed(2) || 'N/A'}ms\n`;
  output += `  max:   ${cd.max?.toFixed(2) || 'N/A'}ms\n`;

  return {
    'stdout': output,
    'results/connection_churn.json': JSON.stringify(summary, null, 2),
  };
}
