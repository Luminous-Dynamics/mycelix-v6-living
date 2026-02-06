/**
 * WebSocket Transport Tests
 */

import { WebSocketTransport, TransportConfig, DEFAULT_TRANSPORT_CONFIG } from '../src/transport';

// Mock WebSocket for testing
class MockWebSocket {
  static instances: MockWebSocket[] = [];
  url: string;
  onopen: (() => void) | null = null;
  onclose: ((event: { code: number }) => void) | null = null;
  onerror: ((error: Event) => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  readyState = 0; // CONNECTING

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string): void {
    // Mock send
  }

  close(): void {
    this.readyState = 3; // CLOSED
    if (this.onclose) {
      this.onclose({ code: 1000 });
    }
  }

  // Test helpers
  simulateOpen(): void {
    this.readyState = 1; // OPEN
    if (this.onopen) {
      this.onopen();
    }
  }

  simulateMessage(data: unknown): void {
    if (this.onmessage) {
      this.onmessage({ data: JSON.stringify(data) });
    }
  }

  simulateError(): void {
    if (this.onerror) {
      this.onerror(new Event('error'));
    }
  }

  simulateClose(code = 1000): void {
    this.readyState = 3;
    if (this.onclose) {
      this.onclose({ code });
    }
  }
}

// Replace global WebSocket
(global as unknown as { WebSocket: typeof MockWebSocket }).WebSocket = MockWebSocket;

describe('WebSocketTransport', () => {
  beforeEach(() => {
    MockWebSocket.instances = [];
  });

  describe('constructor', () => {
    it('should use default config values', () => {
      const transport = new WebSocketTransport({ url: 'ws://test' });
      expect(transport.getState()).toBe('disconnected');
    });

    it('should accept custom config values', () => {
      const transport = new WebSocketTransport({
        url: 'ws://test',
        reconnectDelayMs: 5000,
        maxReconnectAttempts: 5,
      });
      expect(transport.getState()).toBe('disconnected');
    });
  });

  describe('connect', () => {
    it('should establish connection', async () => {
      const transport = new WebSocketTransport({ url: 'ws://test' });

      const connectPromise = transport.connect();

      // Simulate WebSocket connection
      setTimeout(() => {
        const ws = MockWebSocket.instances[0];
        ws.simulateOpen();
      }, 0);

      await connectPromise;

      expect(transport.isConnected()).toBe(true);
      expect(transport.getState()).toBe('connected');
    });

    it('should not connect if already connected', async () => {
      const transport = new WebSocketTransport({ url: 'ws://test' });

      const connectPromise = transport.connect();
      setTimeout(() => MockWebSocket.instances[0].simulateOpen(), 0);
      await connectPromise;

      // Second connect should return immediately
      await transport.connect();
      expect(MockWebSocket.instances.length).toBe(1);
    });
  });

  describe('disconnect', () => {
    it('should disconnect', async () => {
      const transport = new WebSocketTransport({ url: 'ws://test' });

      const connectPromise = transport.connect();
      setTimeout(() => MockWebSocket.instances[0].simulateOpen(), 0);
      await connectPromise;

      transport.disconnect();

      expect(transport.isConnected()).toBe(false);
      expect(transport.getState()).toBe('disconnected');
    });
  });

  describe('message handling', () => {
    it('should dispatch protocol events', async () => {
      const transport = new WebSocketTransport({ url: 'ws://test' });
      const events: unknown[] = [];

      transport.onProtocolEvent((event) => {
        events.push(event);
      });

      const connectPromise = transport.connect();
      setTimeout(() => MockWebSocket.instances[0].simulateOpen(), 0);
      await connectPromise;

      // Simulate receiving a message
      MockWebSocket.instances[0].simulateMessage({
        type: 'PhaseTransitioned',
        data: { from: 'Shadow', to: 'Composting' },
      });

      expect(events.length).toBe(1);
      expect((events[0] as { type: string }).type).toBe('PhaseTransitioned');
    });
  });

  describe('event subscription', () => {
    it('should allow subscribing to events', () => {
      const transport = new WebSocketTransport({ url: 'ws://test' });
      const stateChanges: string[] = [];

      const unsubscribe = transport.on('stateChange', (event) => {
        if (event.type === 'stateChange') {
          stateChanges.push(event.state);
        }
      });

      transport.connect();

      expect(stateChanges).toContain('connecting');

      unsubscribe();
    });

    it('should allow unsubscribing', () => {
      const transport = new WebSocketTransport({ url: 'ws://test' });
      let callCount = 0;

      const unsubscribe = transport.on('stateChange', () => {
        callCount++;
      });

      transport.connect(); // connecting state
      unsubscribe();

      // Further state changes should not trigger callback
      transport.disconnect();

      expect(callCount).toBe(1);
    });
  });
});

describe('DEFAULT_TRANSPORT_CONFIG', () => {
  it('should have reasonable defaults', () => {
    expect(DEFAULT_TRANSPORT_CONFIG.reconnectDelayMs).toBe(1000);
    expect(DEFAULT_TRANSPORT_CONFIG.maxReconnectAttempts).toBe(10);
    expect(DEFAULT_TRANSPORT_CONFIG.heartbeatIntervalMs).toBe(30000);
    expect(DEFAULT_TRANSPORT_CONFIG.connectionTimeoutMs).toBe(10000);
  });
});
