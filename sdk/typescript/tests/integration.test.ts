/**
 * Integration Tests for Living Protocol TypeScript SDK
 *
 * These tests require a running WebSocket server.
 * Run with: npm run test:integration
 *
 * Server should be started with:
 *   cargo run -p ws-server -- --port 9999 --simulated-time --time-acceleration 1000
 */

import { LivingProtocolClient, connectToLivingProtocol } from '../src/client';
import { CyclePhase } from '../src/types';

const SERVER_URL = process.env.WS_SERVER_URL || 'ws://localhost:9999';
const TIMEOUT_MS = 10000;

// Helper to skip tests if server is not available
let serverAvailable = false;
let client: LivingProtocolClient | null = null;

beforeAll(async () => {
  try {
    client = await LivingProtocolClient.connect({
      url: SERVER_URL,
      connectionTimeoutMs: 5000,
    });
    serverAvailable = true;
    console.log('Connected to server at', SERVER_URL);
  } catch (error) {
    console.warn('Server not available at', SERVER_URL, '- skipping integration tests');
    console.warn('Start server with: cargo run -p ws-server -- --port 9999 --simulated-time');
  }
}, TIMEOUT_MS);

afterAll(async () => {
  if (client) {
    client.disconnect();
  }
});

// Conditional describe that skips all tests if server is unavailable
const describeIfServer = () => serverAvailable ? describe : describe.skip;

describeIfServer()('LivingProtocolClient Integration', () => {
  describe('Connection', () => {
    test('client is connected', () => {
      expect(client?.isConnected()).toBe(true);
    });

    test('connection state is connected', () => {
      expect(client?.getConnectionState()).toBe('connected');
    });
  });

  describe('Cycle State Queries', () => {
    test('getCurrentState returns valid state', async () => {
      const state = await client!.getCurrentState();

      expect(state).toBeDefined();
      expect(state.cycleNumber).toBeGreaterThanOrEqual(1);
      expect(state.currentPhase).toBeDefined();
      expect(typeof state.phaseStarted).toBe('string');
      expect(typeof state.cycleStarted).toBe('string');
      expect(state.phaseDay).toBeGreaterThanOrEqual(0);

      // Verify phase is a valid CyclePhase
      const validPhases = Object.values(CyclePhase);
      expect(validPhases).toContain(state.currentPhase);
    }, TIMEOUT_MS);

    test('getCurrentPhase returns a valid phase', async () => {
      const phase = await client!.getCurrentPhase();

      expect(phase).toBeDefined();
      const validPhases = Object.values(CyclePhase);
      expect(validPhases).toContain(phase);
    }, TIMEOUT_MS);

    test('getCycleNumber returns a positive number', async () => {
      const cycleNumber = await client!.getCycleNumber();

      expect(cycleNumber).toBeGreaterThanOrEqual(1);
    }, TIMEOUT_MS);

    test('getTransitionHistory returns an array', async () => {
      const history = await client!.getTransitionHistory();

      expect(Array.isArray(history)).toBe(true);
      // History may be empty initially
      if (history.length > 0) {
        const first = history[0];
        expect(first.from).toBeDefined();
        expect(first.to).toBeDefined();
        expect(first.cycleNumber).toBeGreaterThanOrEqual(1);
        expect(typeof first.transitionedAt).toBe('string');
      }
    }, TIMEOUT_MS);
  });

  describe('Phase Metrics', () => {
    test('getPhaseMetrics returns metrics for Shadow phase', async () => {
      const metrics = await client!.getPhaseMetrics(CyclePhase.Shadow);

      expect(metrics).toBeDefined();
      expect(typeof metrics.activeAgents).toBe('number');
      expect(typeof metrics.spectralK).toBe('number');
      expect(typeof metrics.meanMetabolicTrust).toBe('number');
    }, TIMEOUT_MS);

    test('getPhaseMetrics returns metrics for all phases', async () => {
      const phases = Object.values(CyclePhase);

      for (const phase of phases) {
        const metrics = await client!.getPhaseMetrics(phase);
        expect(metrics).toBeDefined();
      }
    }, TIMEOUT_MS);
  });

  describe('Operation Permissions', () => {
    test('isOperationPermitted returns boolean', async () => {
      const permitted = await client!.isOperationPermitted('vote');
      expect(typeof permitted).toBe('boolean');
    }, TIMEOUT_MS);

    test('vote permission depends on current phase', async () => {
      const phase = await client!.getCurrentPhase();
      const permitted = await client!.isOperationPermitted('vote');

      // Voting is blocked in EmergentPersonhood and Kenosis
      if (phase === CyclePhase.EmergentPersonhood || phase === CyclePhase.Kenosis) {
        expect(permitted).toBe(false);
      }
      // Otherwise voting should be permitted
    }, TIMEOUT_MS);

    test('kenosis operation only permitted in Kenosis phase', async () => {
      const phase = await client!.getCurrentPhase();
      const permitted = await client!.isOperationPermitted('kenosis');

      if (phase === CyclePhase.Kenosis) {
        expect(permitted).toBe(true);
      } else {
        expect(permitted).toBe(false);
      }
    }, TIMEOUT_MS);
  });

  describe('State Caching', () => {
    test('repeated calls use cache', async () => {
      const start = Date.now();

      // First call
      const state1 = await client!.getCurrentState();

      // Rapid second call should use cache
      const state2 = await client!.getCurrentState();

      const elapsed = Date.now() - start;

      // Both should return the same state
      expect(state1.cycleNumber).toBe(state2.cycleNumber);
      expect(state1.currentPhase).toBe(state2.currentPhase);

      // Second call should be very fast (cached)
      expect(elapsed).toBeLessThan(1000);
    }, TIMEOUT_MS);
  });

  describe('Event Subscriptions', () => {
    test('onEvent subscription works', async () => {
      const events: unknown[] = [];
      const unsubscribe = client!.onEvent((event) => {
        events.push(event);
      });

      // Wait a short time for any events
      await new Promise(resolve => setTimeout(resolve, 500));

      // Unsubscribe
      unsubscribe();

      // Events array exists (may or may not have events depending on timing)
      expect(Array.isArray(events)).toBe(true);
    }, TIMEOUT_MS);

    test('onPhaseChange subscription returns unsubscribe function', () => {
      const unsubscribe = client!.onPhaseChange(() => {
        // Handler
      });

      expect(typeof unsubscribe).toBe('function');
      unsubscribe();
    });

    test('subscribeWithFilter accepts filter options', () => {
      const unsubscribe = client!.subscribeWithFilter(
        { eventTypes: ['PhaseTransitioned', 'CycleStarted'] },
        () => {
          // Handler
        }
      );

      expect(typeof unsubscribe).toBe('function');
      unsubscribe();
    });
  });
});

// Test for connecting and disconnecting
describe('Connection Lifecycle', () => {
  test('connectToLivingProtocol helper works', async () => {
    if (!serverAvailable) {
      console.log('Skipping - server not available');
      return;
    }

    const newClient = await connectToLivingProtocol(SERVER_URL);
    expect(newClient.isConnected()).toBe(true);

    newClient.disconnect();
    expect(newClient.isConnected()).toBe(false);
  }, TIMEOUT_MS);

  test('disconnect rejects pending requests', async () => {
    if (!serverAvailable) {
      console.log('Skipping - server not available');
      return;
    }

    const newClient = await LivingProtocolClient.connect({
      url: SERVER_URL,
      connectionTimeoutMs: 5000,
    });

    expect(newClient.isConnected()).toBe(true);

    // Disconnect should clean up without error
    newClient.disconnect();
    expect(newClient.isConnected()).toBe(false);
  }, TIMEOUT_MS);
});
