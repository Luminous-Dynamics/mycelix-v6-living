/**
 * Admin API Client
 * Uses the Living Protocol SDK types and communicates with the admin server.
 */

// Types matching the SDK and server responses
export enum CyclePhase {
  Shadow = 'Shadow',
  Composting = 'Composting',
  Liminal = 'Liminal',
  NegativeCapability = 'NegativeCapability',
  Eros = 'Eros',
  CoCreation = 'CoCreation',
  Beauty = 'Beauty',
  EmergentPersonhood = 'EmergentPersonhood',
  Kenosis = 'Kenosis',
}

export interface CycleState {
  cycleNumber: number;
  currentPhase: CyclePhase;
  phaseStarted: string;
  cycleStarted: string;
  phaseDay: number;
}

export interface PhaseMetrics {
  activeAgents: number;
  spectralK: number;
  meanMetabolicTrust: number;
  activeWounds: number;
  compostingEntities: number;
  liminalEntities: number;
  entangledPairs: number;
  heldUncertainties: number;
}

export interface PhaseTransition {
  from: CyclePhase;
  to: CyclePhase;
  cycleNumber: number;
  transitionedAt: string;
}

export interface ServerMetrics {
  activeConnections: number;
  totalConnections: number;
  messagesReceived: number;
  messagesSent: number;
  uptimeSeconds: number;
}

export interface ConnectionInfo {
  id: number;
  remoteAddr: string;
  connectedAt: string;
  authenticated: boolean;
  messagesReceived: number;
  messagesSent: number;
}

export interface ServerConfig {
  bindAddr: string;
  healthAddr: string | null;
  maxConnections: number;
  maxConnectionsPerIp: number;
  rateLimit: number;
  rateLimitBurst: number;
  authRequired: boolean;
  testMode: boolean;
}

const API_BASE = '/admin/api';

class AdminApiClient {
  private password: string | null = null;

  setPassword(password: string) {
    this.password = password;
  }

  private getHeaders(): HeadersInit {
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
    };
    if (this.password) {
      headers['Authorization'] = `Basic ${btoa(`admin:${this.password}`)}`;
    }
    return headers;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      headers: {
        ...this.getHeaders(),
        ...options.headers,
      },
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || `HTTP ${response.status}`);
    }

    return response.json();
  }

  // Cycle State
  async getCycleState(): Promise<CycleState> {
    return this.request<CycleState>('/state');
  }

  async getPhaseMetrics(phase?: CyclePhase): Promise<PhaseMetrics> {
    const url = phase ? `/metrics/${phase}` : '/metrics';
    return this.request<PhaseMetrics>(url);
  }

  async advanceCycle(): Promise<{ success: boolean; newPhase: CyclePhase }> {
    return this.request<{ success: boolean; newPhase: CyclePhase }>(
      '/cycle/advance',
      { method: 'POST' }
    );
  }

  // Connections
  async getConnections(): Promise<ConnectionInfo[]> {
    return this.request<ConnectionInfo[]>('/connections');
  }

  // Server Metrics
  async getServerMetrics(): Promise<ServerMetrics> {
    return this.request<ServerMetrics>('/server/metrics');
  }

  // History
  async getTransitionHistory(): Promise<PhaseTransition[]> {
    return this.request<PhaseTransition[]>('/history');
  }

  // Settings
  async getConfig(): Promise<ServerConfig> {
    return this.request<ServerConfig>('/config');
  }
}

export const apiClient = new AdminApiClient();
