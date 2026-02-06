// Shared configuration for k6 load tests
// Mycelix Living Protocol WebSocket RPC Server

export const config = {
  // Server connection settings
  server: {
    host: __ENV.WS_HOST || 'localhost',
    port: __ENV.WS_PORT || '8888',
    get url() {
      return `ws://${this.host}:${this.port}`;
    },
    healthPort: __ENV.HEALTH_PORT || '8889',
    get healthUrl() {
      return `http://${this.host}:${this.healthPort}`;
    },
  },

  // Test duration settings
  durations: {
    warmup: __ENV.WARMUP_DURATION || '30s',
    rampUp: __ENV.RAMP_UP_DURATION || '1m',
    steady: __ENV.STEADY_DURATION || '5m',
    rampDown: __ENV.RAMP_DOWN_DURATION || '30s',
  },

  // Load levels
  load: {
    // WebSocket load test
    websocket: {
      maxVUs: parseInt(__ENV.MAX_VUS) || 1000,
      stages: [
        { duration: '30s', target: 100 },   // Warm up
        { duration: '1m', target: 500 },    // Ramp up
        { duration: '2m', target: 1000 },   // Full load
        { duration: '5m', target: 1000 },   // Steady state
        { duration: '1m', target: 500 },    // Ramp down
        { duration: '30s', target: 0 },     // Cool down
      ],
    },
    // Throughput test (single connection)
    throughput: {
      iterations: parseInt(__ENV.ITERATIONS) || 10000,
      duration: __ENV.THROUGHPUT_DURATION || '2m',
    },
    // Connection churn test
    churn: {
      connectionRate: parseInt(__ENV.CONN_RATE) || 50,  // connections per second
      duration: __ENV.CHURN_DURATION || '5m',
    },
  },

  // Performance thresholds
  thresholds: {
    // Latency thresholds (in milliseconds)
    latency: {
      p50: parseInt(__ENV.P50_THRESHOLD) || 10,    // 50th percentile
      p95: parseInt(__ENV.P95_THRESHOLD) || 50,    // 95th percentile
      p99: parseInt(__ENV.P99_THRESHOLD) || 100,   // 99th percentile
    },
    // Error rate threshold (percentage)
    errorRate: parseFloat(__ENV.ERROR_RATE_THRESHOLD) || 0.1,  // 0.1%
    // Throughput thresholds
    throughput: {
      minRps: parseInt(__ENV.MIN_RPS) || 1000,  // Minimum requests per second
    },
  },

  // RPC methods to test
  rpcMethods: {
    getCycleState: {
      method: 'getCycleState',
      params: {},
      weight: 40,  // 40% of requests
    },
    getCurrentPhase: {
      method: 'getCurrentPhase',
      params: {},
      weight: 30,  // 30% of requests
    },
    getCycleNumber: {
      method: 'getCycleNumber',
      params: {},
      weight: 15,  // 15% of requests
    },
    getTransitionHistory: {
      method: 'getTransitionHistory',
      params: {},
      weight: 10,  // 10% of requests
    },
    getPhaseMetrics: {
      method: 'getPhaseMetrics',
      params: { phase: 'Shadow' },
      weight: 5,   // 5% of requests
    },
  },

  // Phases to test (for getPhaseMetrics)
  phases: [
    'Shadow',
    'Composting',
    'Liminal',
    'NegativeCapability',
    'Eros',
    'CoCreation',
    'Beauty',
    'EmergentPersonhood',
    'Kenosis',
  ],
};

// Generate a weighted random RPC method
export function getRandomRpcMethod() {
  const methods = Object.values(config.rpcMethods);
  const totalWeight = methods.reduce((sum, m) => sum + m.weight, 0);
  let random = Math.random() * totalWeight;

  for (const method of methods) {
    random -= method.weight;
    if (random <= 0) {
      // Clone to avoid modifying original
      const result = { ...method };
      // For getPhaseMetrics, randomize the phase
      if (method.method === 'getPhaseMetrics') {
        const randomPhase = config.phases[Math.floor(Math.random() * config.phases.length)];
        result.params = { phase: randomPhase };
      }
      return result;
    }
  }

  return methods[0];
}

// Generate a unique request ID
let requestIdCounter = 0;
export function generateRequestId() {
  return `req-${Date.now()}-${++requestIdCounter}`;
}

// Create an RPC request message
export function createRpcRequest(method, params = {}) {
  return JSON.stringify({
    id: generateRequestId(),
    method: method,
    params: params,
  });
}

// Parse an RPC response
export function parseRpcResponse(data) {
  try {
    const response = JSON.parse(data);
    return {
      success: response.result !== undefined,
      error: response.error,
      id: response.id,
      result: response.result,
    };
  } catch (e) {
    return {
      success: false,
      error: { message: `Parse error: ${e.message}` },
    };
  }
}

// K6 thresholds configuration
export function getK6Thresholds() {
  return {
    'ws_connecting': ['p(95)<1000'],  // 95% of connections under 1s
    'ws_session_duration': ['p(95)<600000'],  // 95% of sessions under 10min
    'rpc_request_duration': [
      `p(50)<${config.thresholds.latency.p50}`,
      `p(95)<${config.thresholds.latency.p95}`,
      `p(99)<${config.thresholds.latency.p99}`,
    ],
    'rpc_errors': [`rate<${config.thresholds.errorRate / 100}`],
    'checks': ['rate>0.99'],  // 99% of checks pass
  };
}

export default config;
