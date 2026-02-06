// RPC Throughput Test for Mycelix Living Protocol
// Tests maximum requests per second on a single WebSocket connection
//
// Usage:
//   k6 run rpc_throughput.js
//   ITERATIONS=50000 THROUGHPUT_DURATION=5m k6 run rpc_throughput.js

import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend, Gauge } from 'k6/metrics';
import { config, createRpcRequest, parseRpcResponse, getK6Thresholds } from './config.js';

// Custom metrics
const rpcRequestDuration = new Trend('rpc_request_duration', true);
const rpcErrors = new Rate('rpc_errors');
const rpcRequestsTotal = new Counter('rpc_requests_total');
const throughputRate = new Gauge('throughput_rps');
const messagesSent = new Counter('messages_sent');
const messagesReceived = new Counter('messages_received');

// Throughput tracking
let requestsInLastSecond = 0;
let lastSecondTimestamp = Date.now();

// Test configuration
export const options = {
  // Single VU for maximum throughput measurement
  vus: 1,
  duration: config.load.throughput.duration,
  thresholds: {
    ...getK6Thresholds(),
    'throughput_rps': [`value>=${config.thresholds.throughput.minRps}`],
  },
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(50)', 'p(90)', 'p(95)', 'p(99)', 'count'],
};

// Pending requests map for latency tracking
const pendingRequests = new Map();

// RPC methods to cycle through
const methods = [
  { method: 'getCycleState', params: {} },
  { method: 'getCurrentPhase', params: {} },
  { method: 'getCycleNumber', params: {} },
  { method: 'getTransitionHistory', params: {} },
];

export default function() {
  const url = config.server.url;
  let messageIndex = 0;
  let totalRequests = 0;
  let completedRequests = 0;
  let errors = 0;

  const res = ws.connect(url, {}, function(socket) {
    // Handle responses
    socket.on('message', function(data) {
      messagesReceived.add(1);
      completedRequests++;

      const response = parseRpcResponse(data);

      // Skip events (no id)
      if (!response.id) return;

      // Calculate latency
      const startTime = pendingRequests.get(response.id);
      if (startTime) {
        const duration = Date.now() - startTime;
        rpcRequestDuration.add(duration);
        pendingRequests.delete(response.id);
      }

      // Track errors
      if (!response.success) {
        errors++;
        rpcErrors.add(1);
      } else {
        rpcErrors.add(0);
      }

      // Track throughput
      requestsInLastSecond++;
      const now = Date.now();
      if (now - lastSecondTimestamp >= 1000) {
        throughputRate.add(requestsInLastSecond);
        requestsInLastSecond = 0;
        lastSecondTimestamp = now;
      }
    });

    socket.on('error', function(e) {
      console.error('WebSocket error:', e);
      errors++;
    });

    // Fire as many requests as possible
    socket.on('open', function() {
      const targetIterations = config.load.throughput.iterations;

      // Send requests in bursts
      const burstSize = 100;  // Requests per burst
      const burstDelay = 10;  // ms between bursts

      function sendBurst() {
        for (let i = 0; i < burstSize && totalRequests < targetIterations; i++) {
          const methodDef = methods[messageIndex % methods.length];
          const request = createRpcRequest(methodDef.method, methodDef.params);
          const requestId = JSON.parse(request).id;

          pendingRequests.set(requestId, Date.now());
          socket.send(request);
          messagesSent.add(1);
          rpcRequestsTotal.add(1);
          totalRequests++;
          messageIndex++;
        }
      }

      // Initial burst
      sendBurst();

      // Continue sending in bursts using setTimeout (simulated with setInterval + check)
      const interval = socket.setInterval(function() {
        if (totalRequests >= targetIterations) {
          // Wait for all responses
          socket.setTimeout(function() {
            socket.close();
          }, 5000);  // 5 second timeout for remaining responses
          return;
        }
        sendBurst();
      }, burstDelay);
    });

    // Keep connection alive
    socket.setInterval(function() {
      socket.send(JSON.stringify({ type: 'ping' }));
    }, 10000);  // Ping every 10 seconds
  });

  check(res, {
    'WebSocket connected': (r) => r && r.status === 101,
  });
}

export function setup() {
  console.log(`Starting RPC throughput test against ${config.server.url}`);
  console.log(`Target iterations: ${config.load.throughput.iterations}`);
  console.log(`Duration: ${config.load.throughput.duration}`);
  console.log(`Min RPS threshold: ${config.thresholds.throughput.minRps}`);
  return {};
}

export function teardown(data) {
  console.log('RPC throughput test completed');
}

export function handleSummary(data) {
  const dur = data.metrics.rpc_request_duration?.values || {};
  const total = data.metrics.rpc_requests_total?.values?.count || 0;
  const testDuration = data.state?.testRunDurationMs / 1000 || 1;
  const avgRps = total / testDuration;

  const summary = {
    timestamp: new Date().toISOString(),
    test: 'rpc_throughput',
    metrics: {
      total_requests: total,
      test_duration_seconds: testDuration.toFixed(2),
      average_rps: avgRps.toFixed(2),
      peak_rps: data.metrics.throughput_rps?.values?.max || avgRps,
      latency: {
        p50: dur['p(50)'],
        p95: dur['p(95)'],
        p99: dur['p(99)'],
        avg: dur.avg,
        max: dur.max,
      },
      error_rate: data.metrics.rpc_errors?.values?.rate || 0,
    },
    thresholds: {
      min_rps_met: avgRps >= config.thresholds.throughput.minRps,
      p50_met: dur['p(50)'] <= config.thresholds.latency.p50,
      p95_met: dur['p(95)'] <= config.thresholds.latency.p95,
      p99_met: dur['p(99)'] <= config.thresholds.latency.p99,
    },
  };

  let output = '\n=== RPC Throughput Test Summary ===\n\n';
  output += `Total Requests: ${total}\n`;
  output += `Test Duration: ${testDuration.toFixed(2)}s\n`;
  output += `Average RPS: ${avgRps.toFixed(2)}\n`;
  output += `Peak RPS: ${summary.metrics.peak_rps}\n\n`;
  output += 'Latency:\n';
  output += `  p(50): ${dur['p(50)']?.toFixed(2) || 'N/A'}ms\n`;
  output += `  p(95): ${dur['p(95)']?.toFixed(2) || 'N/A'}ms\n`;
  output += `  p(99): ${dur['p(99)']?.toFixed(2) || 'N/A'}ms\n`;
  output += `  avg:   ${dur.avg?.toFixed(2) || 'N/A'}ms\n`;
  output += `  max:   ${dur.max?.toFixed(2) || 'N/A'}ms\n\n`;
  output += `Error Rate: ${((data.metrics.rpc_errors?.values?.rate || 0) * 100).toFixed(2)}%\n`;

  return {
    'stdout': output,
    'results/rpc_throughput.json': JSON.stringify(summary, null, 2),
  };
}
