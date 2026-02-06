/**
 * Subscription Manager Tests
 */

import {
  SubscriptionManager,
  SubscriptionFilter,
  METABOLISM_FILTER,
  CONSCIOUSNESS_FILTER,
  EPISTEMICS_FILTER,
  RELATIONAL_FILTER,
  STRUCTURAL_FILTER,
  CYCLE_FILTER,
} from '../src/subscription';
import { CyclePhase } from '../src/types';
import { LivingProtocolEvent } from '../src/cycle';

describe('SubscriptionManager', () => {
  let manager: SubscriptionManager;

  beforeEach(() => {
    manager = new SubscriptionManager();
  });

  describe('subscribe', () => {
    it('should subscribe and receive events', () => {
      const events: LivingProtocolEvent[] = [];

      manager.subscribe({}, (event) => {
        events.push(event);
      });

      const testEvent: LivingProtocolEvent = {
        type: 'WoundCreated',
        data: { woundId: '123', agentDid: 'did:test:1', severity: 'Minor' },
      };

      manager.dispatch(testEvent);

      expect(events.length).toBe(1);
      expect(events[0]).toEqual(testEvent);
    });

    it('should return unsubscribe function', () => {
      let callCount = 0;

      const unsubscribe = manager.subscribe({}, () => {
        callCount++;
      });

      manager.dispatch({ type: 'CycleStarted', data: { cycleNumber: 1, startedAt: '' } });
      expect(callCount).toBe(1);

      unsubscribe();

      manager.dispatch({ type: 'CycleStarted', data: { cycleNumber: 2, startedAt: '' } });
      expect(callCount).toBe(1);
    });
  });

  describe('event type filtering', () => {
    it('should filter by event type', () => {
      const events: LivingProtocolEvent[] = [];

      manager.subscribeToTypes(['WoundCreated', 'WoundPhaseAdvanced'], (event) => {
        events.push(event);
      });

      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '1', agentDid: 'did:test:1', severity: 'Minor' },
      });

      manager.dispatch({
        type: 'KenosisCommitted',
        data: { commitmentId: '2', releasePercentage: 0.1 },
      });

      manager.dispatch({
        type: 'WoundPhaseAdvanced',
        data: { woundId: '1', from: 'Hemostasis', to: 'Inflammation' },
      });

      expect(events.length).toBe(2);
      expect(events[0].type).toBe('WoundCreated');
      expect(events[1].type).toBe('WoundPhaseAdvanced');
    });
  });

  describe('phase filtering', () => {
    it('should filter by current phase', () => {
      const events: LivingProtocolEvent[] = [];

      manager.subscribeToPhases([CyclePhase.Shadow, CyclePhase.Composting], (event) => {
        events.push(event);
      });

      // Should receive (current phase is Shadow by default)
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '1', agentDid: 'did:test:1', severity: 'Minor' },
      });

      // Change phase to Beauty (not in filter)
      manager.setCurrentPhase(CyclePhase.Beauty);

      // Should not receive
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '2', agentDid: 'did:test:2', severity: 'Minor' },
      });

      // Change phase to Composting
      manager.setCurrentPhase(CyclePhase.Composting);

      // Should receive
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '3', agentDid: 'did:test:3', severity: 'Minor' },
      });

      expect(events.length).toBe(2);
    });
  });

  describe('agent filtering', () => {
    it('should filter by agent DID', () => {
      const events: LivingProtocolEvent[] = [];

      manager.subscribeToAgents(['did:test:1', 'did:test:2'], (event) => {
        events.push(event);
      });

      // Should receive (agentDid matches)
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '1', agentDid: 'did:test:1', severity: 'Minor' },
      });

      // Should not receive (agentDid doesn't match)
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '2', agentDid: 'did:test:3', severity: 'Minor' },
      });

      // Should receive (agentA matches)
      manager.dispatch({
        type: 'EntanglementFormed',
        data: { agentA: 'did:test:2', agentB: 'did:test:3', strength: 0.8 },
      });

      expect(events.length).toBe(2);
    });
  });

  describe('custom filter', () => {
    it('should apply custom filter function', () => {
      const events: LivingProtocolEvent[] = [];

      manager.subscribe(
        {
          customFilter: (event) => {
            if (event.type === 'WoundCreated') {
              return event.data.severity === 'Critical';
            }
            return true;
          },
        },
        (event) => {
          events.push(event);
        }
      );

      // Should not receive (severity is Minor)
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '1', agentDid: 'did:test:1', severity: 'Minor' },
      });

      // Should receive (severity is Critical)
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '2', agentDid: 'did:test:1', severity: 'Critical' },
      });

      // Should receive (different event type, passes custom filter)
      manager.dispatch({
        type: 'CycleStarted',
        data: { cycleNumber: 1, startedAt: '' },
      });

      expect(events.length).toBe(2);
    });
  });

  describe('combined filters', () => {
    it('should combine multiple filter criteria', () => {
      const events: LivingProtocolEvent[] = [];

      manager.subscribe(
        {
          eventTypes: ['WoundCreated', 'WoundPhaseAdvanced'],
          agentDids: ['did:test:1'],
          phases: [CyclePhase.Shadow],
        },
        (event) => {
          events.push(event);
        }
      );

      // Matches all criteria
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '1', agentDid: 'did:test:1', severity: 'Minor' },
      });

      // Wrong event type
      manager.dispatch({
        type: 'KenosisCommitted',
        data: { commitmentId: '1', releasePercentage: 0.1 },
      });

      // Wrong agent
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '2', agentDid: 'did:test:2', severity: 'Minor' },
      });

      // Change phase
      manager.setCurrentPhase(CyclePhase.Beauty);

      // Wrong phase now
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '3', agentDid: 'did:test:1', severity: 'Minor' },
      });

      expect(events.length).toBe(1);
    });
  });

  describe('phase transition handling', () => {
    it('should update current phase on PhaseTransitioned event', () => {
      const events: LivingProtocolEvent[] = [];

      manager.subscribeToPhases([CyclePhase.Composting], (event) => {
        events.push(event);
      });

      // Should not receive (current phase is Shadow)
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '1', agentDid: 'did:test:1', severity: 'Minor' },
      });

      // Dispatch phase transition
      manager.dispatch({
        type: 'PhaseTransitioned',
        data: {
          from: CyclePhase.Shadow,
          to: CyclePhase.Composting,
          cycleNumber: 1,
          transitionedAt: '',
          metrics: {} as any,
        },
      });

      // Should receive now (current phase is Composting)
      manager.dispatch({
        type: 'WoundCreated',
        data: { woundId: '2', agentDid: 'did:test:1', severity: 'Minor' },
      });

      expect(events.length).toBe(2); // PhaseTransitioned + WoundCreated
    });
  });

  describe('utility methods', () => {
    it('should track subscription count', () => {
      expect(manager.getSubscriptionCount()).toBe(0);

      const unsub1 = manager.subscribe({}, () => {});
      expect(manager.getSubscriptionCount()).toBe(1);

      const unsub2 = manager.subscribe({}, () => {});
      expect(manager.getSubscriptionCount()).toBe(2);

      unsub1();
      expect(manager.getSubscriptionCount()).toBe(1);

      unsub2();
      expect(manager.getSubscriptionCount()).toBe(0);
    });

    it('should clear all subscriptions', () => {
      manager.subscribe({}, () => {});
      manager.subscribe({}, () => {});
      manager.subscribe({}, () => {});

      expect(manager.getSubscriptionCount()).toBe(3);

      manager.clearAll();

      expect(manager.getSubscriptionCount()).toBe(0);
    });
  });
});

describe('Pre-built filters', () => {
  it('METABOLISM_FILTER should include metabolism events', () => {
    expect(METABOLISM_FILTER.eventTypes).toContain('WoundCreated');
    expect(METABOLISM_FILTER.eventTypes).toContain('KenosisCommitted');
    expect(METABOLISM_FILTER.eventTypes).toContain('MetabolicTrustUpdated');
    expect(METABOLISM_FILTER.eventTypes).toContain('CompostingStarted');
  });

  it('CONSCIOUSNESS_FILTER should include consciousness events', () => {
    expect(CONSCIOUSNESS_FILTER.eventTypes).toContain('TemporalKVectorUpdated');
    expect(CONSCIOUSNESS_FILTER.eventTypes).toContain('NetworkPhiComputed');
    expect(CONSCIOUSNESS_FILTER.eventTypes).toContain('DreamStateChanged');
  });

  it('EPISTEMICS_FILTER should include epistemics events', () => {
    expect(EPISTEMICS_FILTER.eventTypes).toContain('ShadowSurfaced');
    expect(EPISTEMICS_FILTER.eventTypes).toContain('BeautyScored');
    expect(EPISTEMICS_FILTER.eventTypes).toContain('ClaimHeldInUncertainty');
  });

  it('RELATIONAL_FILTER should include relational events', () => {
    expect(RELATIONAL_FILTER.eventTypes).toContain('EntanglementFormed');
    expect(RELATIONAL_FILTER.eventTypes).toContain('LiminalTransitionStarted');
  });

  it('STRUCTURAL_FILTER should include structural events', () => {
    expect(STRUCTURAL_FILTER.eventTypes).toContain('FractalPatternReplicated');
    expect(STRUCTURAL_FILTER.eventTypes).toContain('MycelialTaskDistributed');
  });

  it('CYCLE_FILTER should include cycle events', () => {
    expect(CYCLE_FILTER.eventTypes).toContain('PhaseTransitioned');
    expect(CYCLE_FILTER.eventTypes).toContain('CycleStarted');
  });
});
