/**
 * Jest test setup file.
 *
 * This file runs before each test file and sets up mocks and global configuration.
 */

// Mock the Holochain client
jest.mock('@holochain/client', () => ({
  AppAgentWebsocket: {
    connect: jest.fn().mockResolvedValue({
      appInfo: jest.fn().mockResolvedValue({
        installed_app_id: 'mycelix-living-protocol',
        cell_info: {},
      }),
      callZome: jest.fn(),
    }),
  },
}));

// Global test utilities
(global as any).createMockRecord = (entry: unknown) => ({
  signed_action: {
    hashed: {
      hash: new Uint8Array(32),
      content: {
        author: new Uint8Array(32),
        timestamp: Date.now() * 1000,
        action_seq: 0,
        prev_action: new Uint8Array(32),
      },
    },
    signature: new Uint8Array(64),
  },
  entry: {
    Present: {
      entry_type: 'App',
      entry: entry,
    },
  },
});

// Extend Jest matchers
expect.extend({
  toBeUnitInterval(received: number) {
    const pass = received >= 0 && received <= 1;
    return {
      pass,
      message: () =>
        pass
          ? `expected ${received} not to be in [0, 1]`
          : `expected ${received} to be in [0, 1]`,
    };
  },
});

// Type augmentation for custom matchers
declare global {
  namespace jest {
    interface Matchers<R> {
      toBeUnitInterval(): R;
    }
  }

  function createMockRecord(entry: unknown): unknown;
}

export {};
