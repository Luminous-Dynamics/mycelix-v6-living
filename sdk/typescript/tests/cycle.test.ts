/**
 * Tests for the Cycle SDK module.
 */

import {
  CyclePhase,
  PHASE_DURATIONS,
  TOTAL_CYCLE_DAYS,
} from '../src/cycle';

describe('Cycle Constants', () => {
  describe('CyclePhase', () => {
    it('should have 9 phases', () => {
      const phases = Object.values(CyclePhase);
      expect(phases).toHaveLength(9);
    });

    it('should have correct phase names', () => {
      expect(CyclePhase.Shadow).toBe('Shadow');
      expect(CyclePhase.Composting).toBe('Composting');
      expect(CyclePhase.Liminal).toBe('Liminal');
      expect(CyclePhase.NegativeCapability).toBe('NegativeCapability');
      expect(CyclePhase.Eros).toBe('Eros');
      expect(CyclePhase.CoCreation).toBe('CoCreation');
      expect(CyclePhase.Beauty).toBe('Beauty');
      expect(CyclePhase.EmergentPersonhood).toBe('EmergentPersonhood');
      expect(CyclePhase.Kenosis).toBe('Kenosis');
    });
  });

  describe('Phase Durations', () => {
    it('should sum to 28 days', () => {
      expect(TOTAL_CYCLE_DAYS).toBe(28);

      const totalDays = (Object.values(PHASE_DURATIONS) as number[]).reduce(
        (sum, days) => sum + days,
        0
      );
      expect(totalDays).toBe(28);
    });

    it('should have all phases with durations', () => {
      Object.values(CyclePhase).forEach((phase) => {
        expect(PHASE_DURATIONS[phase]).toBeDefined();
        expect(PHASE_DURATIONS[phase]).toBeGreaterThan(0);
      });
    });
  });
});

describe('Cycle Calculations', () => {
  const getPhaseFoDay = (day: number): CyclePhase => {
    let accumulated = 0;
    for (const [phase, duration] of Object.entries(PHASE_DURATIONS)) {
      accumulated += duration as number;
      if (day <= accumulated) {
        return phase as CyclePhase;
      }
    }
    return CyclePhase.Kenosis;
  };

  it('should determine correct phase for day 1', () => {
    expect(getPhaseFoDay(1)).toBe(CyclePhase.Shadow);
  });

  it('should determine correct phase for last day of Shadow', () => {
    const shadowEnd = PHASE_DURATIONS[CyclePhase.Shadow];
    expect(getPhaseFoDay(shadowEnd)).toBe(CyclePhase.Shadow);
  });

  it('should determine correct phase for first day of Composting', () => {
    const compostingStart = PHASE_DURATIONS[CyclePhase.Shadow] + 1;
    expect(getPhaseFoDay(compostingStart)).toBe(CyclePhase.Composting);
  });

  it('should determine correct phase for day 28', () => {
    expect(getPhaseFoDay(28)).toBe(CyclePhase.Kenosis);
  });

  it('should calculate days remaining in phase', () => {
    const currentPhaseDay = 1;
    const currentPhase = CyclePhase.Shadow;
    const phaseDuration = PHASE_DURATIONS[currentPhase]; // Shadow = 2 days

    const daysRemaining = phaseDuration - currentPhaseDay;
    expect(daysRemaining).toBe(1); // 2 - 1 = 1
  });

  it('should calculate total days elapsed', () => {
    // Day 2 of Composting phase
    const phasesSoFar = [CyclePhase.Shadow];
    const dayInCurrentPhase = 2;

    let totalDays = 0;
    phasesSoFar.forEach((phase) => {
      totalDays += PHASE_DURATIONS[phase];
    });
    totalDays += dayInCurrentPhase;

    expect(totalDays).toBe(4); // 2 (Shadow) + 2 (Composting day 2)
  });
});

describe('Cycle Invariants', () => {
  it('should have phases in fixed order', () => {
    const expectedOrder = [
      CyclePhase.Shadow,
      CyclePhase.Composting,
      CyclePhase.Liminal,
      CyclePhase.NegativeCapability,
      CyclePhase.Eros,
      CyclePhase.CoCreation,
      CyclePhase.Beauty,
      CyclePhase.EmergentPersonhood,
      CyclePhase.Kenosis,
    ];

    const actualOrder = Object.keys(PHASE_DURATIONS);
    expect(actualOrder).toEqual(expectedOrder);
  });

  it('should wrap from Kenosis back to Shadow', () => {
    const phases = Object.keys(PHASE_DURATIONS) as CyclePhase[];
    const lastPhase = phases[phases.length - 1];
    const firstPhase = phases[0];

    expect(lastPhase).toBe(CyclePhase.Kenosis);
    expect(firstPhase).toBe(CyclePhase.Shadow);
  });

  it('should have positive durations for all phases', () => {
    Object.values(PHASE_DURATIONS).forEach((duration) => {
      expect(duration).toBeGreaterThan(0);
    });
  });
});
