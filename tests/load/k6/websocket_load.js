// WebSocket Load Test for Mycelix Living Protocol
// Tests concurrent WebSocket connections with RPC requests
//
// Usage:
//   k6 run websocket_load.js
//   k6 run --vus 100 --duration 1m websocket_load.js
//   WS_HOST=myserver.com WS_PORT=8888 k6 run websocket_load.js

import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';
import { config, getRandomRpcMethod, createRpcRequest, parseRpcResponse, getK6Thresholds } from './config.js';

// Custom metrics
const rpcRequestDuration = new Trend('rpc_request_duration', true);
const rpcErrors = new Rate('rpc_errors');
const rpcRequestsTotal = new Counter('rpc_requests_total');
const connectionsOpened = new Counter('connections_opened');
const connectionsFailed = new Counter('connections_failed');
const messagesReceived = new Counter('messages_received');
const messagesSent = new Counter('messages_sent');
const eventsReceived = new Counter('events_received');

// Test configuration
export const options = {
  stages: config.load.websocket.stages,
  thresholds: getK6Thresholds(),
  // Scenario-based configuration for more control
  scenarios: {
    websocket_load: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: config.load.websocket.stages,
      gracefulRampDown: '30s',
    },
  },
  // Summary configuration
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(50)', 'p(90)', 'p(95)', 'p(99)', 'count'],
};

// Track pending requests for latency measurement
const pendingRequests = new Map();

export default function() {
  const url = config.server.url;
  const params = {
    tags: { name: 'websocket_load' },
  };

  const res = ws.connect(url, params, function(socket) {
    connectionsOpened.add(1);

    // Handle incoming messages
    socket.on('message', function(data) {
      messagesReceived.add(1);

      const response = parseRpcResponse(data);

      // Check if this is an event (no id) or response
      if (!response.id) {
        eventsReceived.add(1);
        return;
      }

      // Find the pending request to calculate latency
      const startTime = pendingRequests.get(response.id);
      if (startTime) {
        const duration = Date.now() - startTime;
        rpcRequestDuration.add(duration);
        pendingRequests.delete(response.id);
      }

      // Track errors
      if (!response.success) {
        rpcErrors.add(1);
      } else {
        rpcErrors.add(0);
      }
    });

    socket.on('error', function(e) {
      console.error('WebSocket error:', e);
      rpcErrors.add(1);
    });

    socket.on('close', function() {
      // Connection closed
    });

    socket.on('open', function() {
      // Send initial getCycleState request
      const initRequest = createRpcRequest('getCycleState');
      const initId = JSON.parse(initRequest).id;
      pendingRequests.set(initId, Date.now());
      socket.send(initRequest);
      messagesSent.add(1);
      rpcRequestsTotal.add(1);
    });

    // Send RPC requests at regular intervals
    const requestInterval = 1000 + Math.random() * 2000;  // 1-3 seconds
    const sessionDuration = 30000 + Math.random() * 60000;  // 30-90 seconds

    let elapsed = 0;
    while (elapsed < sessionDuration) {
      // Send a random RPC request
      const rpcMethod = getRandomRpcMethod();
      const request = createRpcRequest(rpcMethod.method, rpcMethod.params);
      const requestId = JSON.parse(request).id;

      pendingRequests.set(requestId, Date.now());
      socket.send(request);
      messagesSent.add(1);
      rpcRequestsTotal.add(1);

      sleep(requestInterval / 1000);
      elapsed += requestInterval;
    }

    // Graceful close
    socket.close();
  });

  // Check connection success
  const connected = check(res, {
    'WebSocket connected successfully': (r) => r && r.status === 101,
  });

  if (!connected) {
    connectionsFailed.add(1);
  }

  // Small delay between VU iterations
  sleep(Math.random() * 2);
}

// Setup: verify server is running
export function setup() {
  console.log(`Starting WebSocket load test against ${config.server.url}`);
  console.log(`Max VUs: ${config.load.websocket.maxVUs}`);
  console.log(`Latency thresholds: p50=${config.thresholds.latency.p50}ms, p95=${config.thresholds.latency.p95}ms, p99=${config.thresholds.latency.p99}ms`);

  // Note: k6 doesn't support HTTP in WebSocket tests setup directly
  // Server health check should be done externally before running tests
  return {};
}

// Teardown: log summary
export function teardown(data) {
  console.log('WebSocket load test completed');
}

// Handle summary output
export function handleSummary(data) {
  // Extract key metrics for CI/CD integration
  const summary = {
    timestamp: new Date().toISOString(),
    test: 'websocket_load',
    metrics: {
      vus_max: data.metrics.vus_max ? data.metrics.vus_max.values.max : 0,
      rpc_requests_total: data.metrics.rpc_requests_total ? data.metrics.rpc_requests_total.values.count : 0,
      rpc_request_duration_p50: data.metrics.rpc_request_duration ? data.metrics.rpc_request_duration.values['p(50)'] : null,
      rpc_request_duration_p95: data.metrics.rpc_request_duration ? data.metrics.rpc_request_duration.values['p(95)'] : null,
      rpc_request_duration_p99: data.metrics.rpc_request_duration ? data.metrics.rpc_request_duration.values['p(99)'] : null,
      error_rate: data.metrics.rpc_errors ? data.metrics.rpc_errors.values.rate : 0,
      connections_opened: data.metrics.connections_opened ? data.metrics.connections_opened.values.count : 0,
      connections_failed: data.metrics.connections_failed ? data.metrics.connections_failed.values.count : 0,
    },
    thresholds: {
      passed: Object.values(data.root_group.checks || {}).every(c => c.passes > 0 && c.fails === 0),
    },
  };

  return {
    'stdout': textSummary(data, { indent: ' ', enableColors: true }),
    'results/websocket_load.json': JSON.stringify(summary, null, 2),
  };
}

// Helper for text summary (simplified version)
function textSummary(data, options) {
  let output = '\n=== WebSocket Load Test Summary ===\n\n';

  if (data.metrics.rpc_request_duration) {
    const dur = data.metrics.rpc_request_duration.values;
    output += 'RPC Request Duration:\n';
    output += `  p(50): ${dur['p(50)']?.toFixed(2) || 'N/A'}ms\n`;
    output += `  p(95): ${dur['p(95)']?.toFixed(2) || 'N/A'}ms\n`;
    output += `  p(99): ${dur['p(99)']?.toFixed(2) || 'N/A'}ms\n`;
    output += `  avg:   ${dur.avg?.toFixed(2) || 'N/A'}ms\n`;
    output += `  max:   ${dur.max?.toFixed(2) || 'N/A'}ms\n\n`;
  }

  if (data.metrics.rpc_requests_total) {
    output += `Total RPC Requests: ${data.metrics.rpc_requests_total.values.count}\n`;
  }

  if (data.metrics.rpc_errors) {
    output += `Error Rate: ${(data.metrics.rpc_errors.values.rate * 100).toFixed(2)}%\n`;
  }

  if (data.metrics.connections_opened) {
    output += `Connections Opened: ${data.metrics.connections_opened.values.count}\n`;
  }

  if (data.metrics.connections_failed) {
    output += `Connections Failed: ${data.metrics.connections_failed.values.count}\n`;
  }

  return output;
}
