/**
 * Mycelix v6.0 Living Protocol - Example dApp
 *
 * This example demonstrates how to integrate with the Living Protocol
 * using the TypeScript SDK.
 */

import { AppAgentWebsocket } from '@holochain/client';
import {
  CyclePhase,
  PHASE_DURATIONS,
  CompostableEntity,
  WoundSeverity,
  WoundPhase,
} from '@mycelix/living-protocol-sdk';

// Configuration
const HOLOCHAIN_URL = process.env.HOLOCHAIN_URL || 'ws://localhost:8888';
const APP_ID = process.env.APP_ID || 'mycelix-living-protocol';

interface CycleState {
  cycle_number: number;
  current_phase: CyclePhase;
  phase_day: number;
  cycle_started: number;
  phase_started: number;
}

/**
 * Main application class demonstrating Living Protocol integration.
 */
class LivingProtocolDemo {
  private client: AppAgentWebsocket | null = null;

  async connect(): Promise<void> {
    console.log(`Connecting to Holochain at ${HOLOCHAIN_URL}...`);

    try {
      this.client = await AppAgentWebsocket.connect(HOLOCHAIN_URL, APP_ID);
      console.log('Connected successfully!');
    } catch (error) {
      console.error('Failed to connect:', error);
      throw error;
    }
  }

  /**
   * Get the current cycle state.
   */
  async getCycleState(): Promise<CycleState> {
    if (!this.client) throw new Error('Not connected');

    const state = await this.client.callZome({
      role_name: 'mycelix-living-protocol',
      zome_name: 'cycle_engine',
      fn_name: 'get_cycle_state',
      payload: null,
    });

    return state as CycleState;
  }

  /**
   * Demonstrate wound healing workflow.
   */
  async demonstrateWoundHealing(): Promise<void> {
    if (!this.client) throw new Error('Not connected');

    console.log('\n=== Wound Healing Demonstration ===\n');

    // Create a wound
    console.log('Creating a moderate wound...');
    const wound = await this.client.callZome({
      role_name: 'mycelix-living-protocol',
      zome_name: 'living_metabolism',
      fn_name: 'create_wound',
      payload: {
        severity: WoundSeverity.Moderate,
        cause: 'Protocol violation during testing',
        escrow_amount: 100,
      },
    });

    console.log(`Wound created: ${JSON.stringify(wound, null, 2)}`);

    // Advance through phases
    const phases = [
      WoundPhase.Inflammation,
      WoundPhase.Proliferation,
      WoundPhase.Remodeling,
      WoundPhase.Healed,
    ];

    for (const targetPhase of phases) {
      console.log(`\nAdvancing to ${targetPhase}...`);

      const updated = await this.client.callZome({
        role_name: 'mycelix-living-protocol',
        zome_name: 'living_metabolism',
        fn_name: 'advance_wound_phase',
        payload: {
          wound_id: wound.id,
        },
      });

      console.log(`Phase advanced to: ${updated.current_phase}`);
    }

    console.log('\nWound healing complete!');
  }

  /**
   * Demonstrate composting workflow.
   */
  async demonstrateComposting(): Promise<void> {
    if (!this.client) throw new Error('Not connected');

    console.log('\n=== Composting Demonstration ===\n');

    // Start composting a failed proposal
    console.log('Starting composting process...');
    const composting = await this.client.callZome({
      role_name: 'mycelix-living-protocol',
      zome_name: 'living_metabolism',
      fn_name: 'start_composting',
      payload: {
        entity_type: CompostableEntity.FailedProposal,
        entity_id: `proposal-${Date.now()}`,
        reason: 'Quorum not reached',
      },
    });

    console.log(`Composting started: ${JSON.stringify(composting, null, 2)}`);
  }

  /**
   * Demonstrate K-Vector submission.
   */
  async demonstrateKVector(): Promise<void> {
    if (!this.client) throw new Error('Not connected');

    console.log('\n=== K-Vector Demonstration ===\n');

    const dimensions = {
      presence: 0.8,
      coherence: 0.7,
      receptivity: 0.6,
      integration: 0.75,
      generativity: 0.65,
      surrender: 0.5,
      discernment: 0.85,
      emergence: 0.7,
    };

    console.log('Submitting K-Vector snapshot...');
    const snapshot = await this.client.callZome({
      role_name: 'mycelix-living-protocol',
      zome_name: 'living_consciousness',
      fn_name: 'submit_k_vector_snapshot',
      payload: { dimensions },
    });

    console.log(`K-Vector submitted: ${JSON.stringify(snapshot, null, 2)}`);

    // Calculate composite score
    const values = Object.values(dimensions);
    const composite = values.reduce((a, b) => a + b, 0) / values.length;
    console.log(`Composite consciousness score: ${composite.toFixed(3)}`);
  }

  /**
   * Display cycle information.
   */
  async displayCycleInfo(): Promise<void> {
    console.log('\n=== Cycle Information ===\n');

    try {
      const state = await this.getCycleState();

      console.log(`Cycle Number: ${state.cycle_number}`);
      console.log(`Current Phase: ${state.current_phase}`);
      console.log(`Day in Phase: ${state.phase_day}`);

      const phaseDuration = PHASE_DURATIONS[state.current_phase];
      const daysRemaining = phaseDuration - state.phase_day;
      console.log(`Days until next phase: ${daysRemaining}`);

      console.log('\n28-Day Cycle Overview:');
      let dayCount = 0;
      for (const [phase, duration] of Object.entries(PHASE_DURATIONS)) {
        const startDay = dayCount + 1;
        dayCount += duration as number;
        const marker = phase === state.current_phase ? ' <-- Current' : '';
        console.log(`  ${phase}: Day ${startDay}-${dayCount}${marker}`);
      }
    } catch (error) {
      console.log('Could not fetch cycle state (Holochain may not be running)');
      console.log('Displaying static cycle information:\n');

      let dayCount = 0;
      for (const [phase, duration] of Object.entries(PHASE_DURATIONS)) {
        const startDay = dayCount + 1;
        dayCount += duration as number;
        console.log(`  ${phase}: Day ${startDay}-${dayCount} (${duration} days)`);
      }
    }
  }

  /**
   * Run the demo.
   */
  async run(): Promise<void> {
    console.log('='.repeat(60));
    console.log('  Mycelix v6.0 Living Protocol Demo');
    console.log('='.repeat(60));

    // Always display cycle info (works offline)
    await this.displayCycleInfo();

    // Try to connect and run interactive demos
    try {
      await this.connect();

      await this.demonstrateWoundHealing();
      await this.demonstrateComposting();
      await this.demonstrateKVector();

      console.log('\n=== Demo Complete ===\n');
    } catch (error) {
      console.log('\nNote: Full demo requires a running Holochain conductor.');
      console.log('Start Holochain with: hc sandbox generate && hc sandbox run');
    }
  }
}

// Main entry point
const demo = new LivingProtocolDemo();
demo.run().catch(console.error);
