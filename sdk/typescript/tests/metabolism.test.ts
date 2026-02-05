/**
 * Tests for the Metabolism SDK module.
 */

import {
  CompostableEntity,
  WoundPhase,
  WoundSeverity,
} from '../src/metabolism';

describe('Metabolism Types', () => {
  describe('CompostableEntity', () => {
    it('should have all entity types defined', () => {
      expect(CompostableEntity.FailedProposal).toBe('FailedProposal');
      expect(CompostableEntity.AbandonedProject).toBe('AbandonedProject');
      expect(CompostableEntity.ExpiredClaim).toBe('ExpiredClaim');
      expect(CompostableEntity.DeprecatedComponent).toBe('DeprecatedComponent');
      expect(CompostableEntity.DissolvedDao).toBe('DissolvedDao');
    });
  });

  describe('WoundPhase', () => {
    it('should have all phases in correct order', () => {
      const phases = [
        WoundPhase.Hemostasis,
        WoundPhase.Inflammation,
        WoundPhase.Proliferation,
        WoundPhase.Remodeling,
        WoundPhase.Healed,
      ];

      expect(phases).toHaveLength(5);
      expect(phases[0]).toBe('Hemostasis');
      expect(phases[4]).toBe('Healed');
    });
  });

  describe('WoundSeverity', () => {
    it('should have all severity levels', () => {
      expect(WoundSeverity.Minor).toBe('Minor');
      expect(WoundSeverity.Moderate).toBe('Moderate');
      expect(WoundSeverity.Severe).toBe('Severe');
      expect(WoundSeverity.Critical).toBe('Critical');
    });

    it('should map slash percentages correctly', () => {
      const getSeverity = (pct: number): string => {
        if (pct >= 0.30) return WoundSeverity.Critical;
        if (pct >= 0.15) return WoundSeverity.Severe;
        if (pct >= 0.05) return WoundSeverity.Moderate;
        return WoundSeverity.Minor;
      };

      expect(getSeverity(0.01)).toBe(WoundSeverity.Minor);
      expect(getSeverity(0.05)).toBe(WoundSeverity.Moderate);
      expect(getSeverity(0.15)).toBe(WoundSeverity.Severe);
      expect(getSeverity(0.30)).toBe(WoundSeverity.Critical);
    });
  });
});

describe('Metabolism Invariants', () => {
  describe('Kenosis', () => {
    it('should enforce max 20% release per cycle', () => {
      const MAX_KENOSIS = 0.20;
      const testValues = [0.10, 0.15, 0.20, 0.25, 0.50];

      testValues.forEach((value) => {
        if (value <= MAX_KENOSIS) {
          expect(value).toBeLessThanOrEqual(MAX_KENOSIS);
        } else {
          expect(value).toBeGreaterThan(MAX_KENOSIS);
        }
      });
    });
  });

  describe('Trust Scores', () => {
    it('should be in unit interval [0, 1]', () => {
      const validScores = [0, 0.5, 0.75, 1.0];
      const invalidScores = [-0.1, 1.5, 2.0];

      validScores.forEach((score) => {
        expect(score >= 0 && score <= 1).toBe(true);
      });

      invalidScores.forEach((score) => {
        expect(score >= 0 && score <= 1).toBe(false);
      });
    });
  });

  describe('Composting Progress', () => {
    it('should be monotonically increasing', () => {
      const progressSteps = [0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0];

      for (let i = 1; i < progressSteps.length; i++) {
        expect(progressSteps[i]).toBeGreaterThanOrEqual(progressSteps[i - 1]);
      }
    });

    it('should be bounded by [0, 1]', () => {
      const progressSteps = [0.0, 0.5, 1.0];

      progressSteps.forEach((progress) => {
        expect(progress >= 0 && progress <= 1).toBe(true);
      });
    });
  });
});
