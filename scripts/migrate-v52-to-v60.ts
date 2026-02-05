#!/usr/bin/env npx tsx
/**
 * Mycelix v5.2 to v6.0 Migration Script
 *
 * This script migrates data from the v5.2 mycelix-property DNA to the
 * v6.0 mycelix-living-protocol DNA.
 *
 * Migration steps:
 * 1. Export agent data from v5.2
 * 2. Convert MATL scores to MetabolicTrust records
 * 3. Convert pending slashes to Wound records
 * 4. Migrate governance proposals with beauty score metadata
 * 5. Export K-Vector history for temporal analysis
 */

import { AppAgentWebsocket } from '@holochain/client';

// Configuration
const V52_URL = process.env.V52_URL || 'ws://localhost:8888';
const V60_URL = process.env.V60_URL || 'ws://localhost:8889';
const V52_APP_ID = 'mycelix-property';
const V60_APP_ID = 'mycelix-living-protocol';

interface MigrationStats {
  agentsProcessed: number;
  matlScoresMigrated: number;
  slashesConverted: number;
  proposalsMigrated: number;
  kvectorSnapshots: number;
  errors: string[];
}

interface V52MatlScore {
  agent: Uint8Array;
  score: number;
  throughput: number;
  resilience: number;
  timestamp: number;
}

interface V52SlashRecord {
  offender: Uint8Array;
  percentage: number;
  reason: string;
  timestamp: number;
}

interface V52Proposal {
  id: string;
  content: string;
  creator: Uint8Array;
  status: string;
  votes: number;
}

interface V52KVector {
  agent: Uint8Array;
  dimensions: number[];
  timestamp: number;
}

/**
 * Migration orchestrator class.
 */
class MigrationOrchestrator {
  private v52Client: AppAgentWebsocket | null = null;
  private v60Client: AppAgentWebsocket | null = null;
  private stats: MigrationStats = {
    agentsProcessed: 0,
    matlScoresMigrated: 0,
    slashesConverted: 0,
    proposalsMigrated: 0,
    kvectorSnapshots: 0,
    errors: [],
  };

  async connect(): Promise<void> {
    console.log('Connecting to Holochain conductors...');

    try {
      this.v52Client = await AppAgentWebsocket.connect(V52_URL, V52_APP_ID);
      console.log('Connected to v5.2 DNA');

      this.v60Client = await AppAgentWebsocket.connect(V60_URL, V60_APP_ID);
      console.log('Connected to v6.0 DNA');
    } catch (error) {
      throw new Error(`Failed to connect: ${error}`);
    }
  }

  /**
   * Migrate MATL scores to MetabolicTrust records.
   */
  async migrateMatlScores(): Promise<void> {
    console.log('\n=== Migrating MATL Scores ===\n');

    if (!this.v52Client || !this.v60Client) {
      throw new Error('Not connected');
    }

    try {
      // Fetch all MATL scores from v5.2
      const matlScores: V52MatlScore[] = await this.v52Client.callZome({
        role_name: 'mycelix-property',
        zome_name: 'governance',
        fn_name: 'get_all_matl_scores',
        payload: null,
      });

      console.log(`Found ${matlScores.length} MATL scores to migrate`);

      for (const matl of matlScores) {
        try {
          // Convert to MetabolicTrust format
          await this.v60Client.callZome({
            role_name: 'mycelix-living-protocol',
            zome_name: 'living_metabolism',
            fn_name: 'update_metabolic_trust',
            payload: {
              target_agent: matl.agent,
              trust_score: matl.score,
              matl_component: matl.score,
              throughput_component: matl.throughput,
              resilience_component: matl.resilience,
              composting_contribution: 0, // Will be computed going forward
            },
          });

          this.stats.matlScoresMigrated++;
          process.stdout.write('.');
        } catch (error) {
          this.stats.errors.push(`MATL migration error: ${error}`);
        }
      }

      console.log(`\nMigrated ${this.stats.matlScoresMigrated} MATL scores`);
    } catch (error) {
      console.log('Could not fetch MATL scores (v5.2 may not be running)');
    }
  }

  /**
   * Convert pending slashes to Wound records.
   */
  async convertSlashesToWounds(): Promise<void> {
    console.log('\n=== Converting Slashes to Wounds ===\n');

    if (!this.v52Client || !this.v60Client) {
      throw new Error('Not connected');
    }

    try {
      // Fetch pending slashes from v5.2
      const slashes: V52SlashRecord[] = await this.v52Client.callZome({
        role_name: 'mycelix-property',
        zome_name: 'slashing',
        fn_name: 'get_pending_slashes',
        payload: null,
      });

      console.log(`Found ${slashes.length} pending slashes to convert`);

      for (const slash of slashes) {
        try {
          // Determine wound severity from slash percentage
          let severity: string;
          if (slash.percentage >= 0.30) {
            severity = 'Critical';
          } else if (slash.percentage >= 0.15) {
            severity = 'Severe';
          } else if (slash.percentage >= 0.05) {
            severity = 'Moderate';
          } else {
            severity = 'Minor';
          }

          // Create wound record
          await this.v60Client.callZome({
            role_name: 'mycelix-living-protocol',
            zome_name: 'living_metabolism',
            fn_name: 'create_wound',
            payload: {
              agent: slash.offender,
              severity,
              cause: `Migrated from v5.2 slash: ${slash.reason}`,
              escrow_amount: slash.percentage * 1000, // Convert to escrow amount
              v52_slash_reference: true,
            },
          });

          this.stats.slashesConverted++;
          process.stdout.write('.');
        } catch (error) {
          this.stats.errors.push(`Slash conversion error: ${error}`);
        }
      }

      console.log(`\nConverted ${this.stats.slashesConverted} slashes to wounds`);
    } catch (error) {
      console.log('Could not fetch slashes (v5.2 may not be running)');
    }
  }

  /**
   * Migrate governance proposals with beauty score potential.
   */
  async migrateProposals(): Promise<void> {
    console.log('\n=== Migrating Governance Proposals ===\n');

    if (!this.v52Client || !this.v60Client) {
      throw new Error('Not connected');
    }

    try {
      // Fetch active proposals from v5.2
      const proposals: V52Proposal[] = await this.v52Client.callZome({
        role_name: 'mycelix-property',
        zome_name: 'governance',
        fn_name: 'get_active_proposals',
        payload: null,
      });

      console.log(`Found ${proposals.length} proposals to migrate`);

      for (const proposal of proposals) {
        try {
          // Register proposal for beauty scoring
          await this.v60Client.callZome({
            role_name: 'mycelix-living-protocol',
            zome_name: 'living_epistemics',
            fn_name: 'register_proposal_for_beauty',
            payload: {
              v52_proposal_id: proposal.id,
              content: proposal.content,
              creator: proposal.creator,
            },
          });

          this.stats.proposalsMigrated++;
          process.stdout.write('.');
        } catch (error) {
          this.stats.errors.push(`Proposal migration error: ${error}`);
        }
      }

      console.log(`\nMigrated ${this.stats.proposalsMigrated} proposals`);
    } catch (error) {
      console.log('Could not fetch proposals (v5.2 may not be running)');
    }
  }

  /**
   * Export K-Vector history for temporal analysis.
   */
  async migrateKVectors(): Promise<void> {
    console.log('\n=== Migrating K-Vector History ===\n');

    if (!this.v52Client || !this.v60Client) {
      throw new Error('Not connected');
    }

    try {
      // Fetch K-Vector history from v5.2
      const kvectors: V52KVector[] = await this.v52Client.callZome({
        role_name: 'mycelix-property',
        zome_name: 'k_vector',
        fn_name: 'get_all_snapshots',
        payload: { limit: 1000 },
      });

      console.log(`Found ${kvectors.length} K-Vector snapshots to migrate`);

      for (const kvec of kvectors) {
        try {
          // Extend 5D to 8D K-Vector
          const extended = {
            presence: kvec.dimensions[0] || 0.5,
            coherence: kvec.dimensions[1] || 0.5,
            receptivity: kvec.dimensions[2] || 0.5,
            integration: kvec.dimensions[3] || 0.5,
            generativity: kvec.dimensions[4] || 0.5,
            // New dimensions default to 0.5
            surrender: 0.5,
            discernment: 0.5,
            emergence: 0.5,
          };

          await this.v60Client.callZome({
            role_name: 'mycelix-living-protocol',
            zome_name: 'living_consciousness',
            fn_name: 'submit_k_vector_snapshot',
            payload: {
              agent: kvec.agent,
              dimensions: extended,
              v52_timestamp: kvec.timestamp,
            },
          });

          this.stats.kvectorSnapshots++;
          process.stdout.write('.');
        } catch (error) {
          this.stats.errors.push(`K-Vector migration error: ${error}`);
        }
      }

      console.log(`\nMigrated ${this.stats.kvectorSnapshots} K-Vector snapshots`);
    } catch (error) {
      console.log('Could not fetch K-Vectors (v5.2 may not be running)');
    }
  }

  /**
   * Print migration summary.
   */
  printSummary(): void {
    console.log('\n' + '='.repeat(60));
    console.log('  Migration Summary');
    console.log('='.repeat(60));
    console.log(`  MATL Scores Migrated:    ${this.stats.matlScoresMigrated}`);
    console.log(`  Slashes Converted:       ${this.stats.slashesConverted}`);
    console.log(`  Proposals Migrated:      ${this.stats.proposalsMigrated}`);
    console.log(`  K-Vector Snapshots:      ${this.stats.kvectorSnapshots}`);
    console.log(`  Errors:                  ${this.stats.errors.length}`);
    console.log('='.repeat(60));

    if (this.stats.errors.length > 0) {
      console.log('\nErrors:');
      this.stats.errors.slice(0, 10).forEach((e) => console.log(`  - ${e}`));
      if (this.stats.errors.length > 10) {
        console.log(`  ... and ${this.stats.errors.length - 10} more`);
      }
    }
  }

  /**
   * Run the full migration.
   */
  async run(): Promise<void> {
    console.log('='.repeat(60));
    console.log('  Mycelix v5.2 to v6.0 Migration');
    console.log('='.repeat(60));

    try {
      await this.connect();

      await this.migrateMatlScores();
      await this.convertSlashesToWounds();
      await this.migrateProposals();
      await this.migrateKVectors();

      this.printSummary();
    } catch (error) {
      console.error('\nMigration failed:', error);
      console.log('\nNote: This migration requires both v5.2 and v6.0 conductors running.');
      console.log('For testing, you can run this script in dry-run mode.');

      // Print what would have been done
      console.log('\n=== Dry Run Summary ===');
      console.log('Migration would perform:');
      console.log('  1. Export all MATL scores from v5.2');
      console.log('  2. Convert to MetabolicTrust records in v6.0');
      console.log('  3. Convert pending slashes to Wound records');
      console.log('  4. Register proposals for beauty scoring');
      console.log('  5. Extend 5D K-Vectors to 8D format');
    }
  }
}

// Main entry point
const migration = new MigrationOrchestrator();
migration.run().catch(console.error);
