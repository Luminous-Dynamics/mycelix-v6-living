#!/usr/bin/env node

/**
 * Living Protocol CLI
 *
 * Command-line interface for monitoring and interacting with the Living Protocol.
 */

import { Command } from 'commander';
import chalk from 'chalk';
import {
  LivingProtocolClient,
  CyclePhase,
  CycleState,
  PhaseTransition,
  PhaseMetrics,
  LivingProtocolEvent,
  PHASE_ORDER,
} from '@mycelix/living-protocol-sdk';

// Phase colors for terminal display
const PHASE_COLORS: Record<CyclePhase, (text: string) => string> = {
  [CyclePhase.Shadow]: chalk.hex('#4a4a6a'),
  [CyclePhase.Composting]: chalk.hex('#6b4423'),
  [CyclePhase.Liminal]: chalk.hex('#7c4dff'),
  [CyclePhase.NegativeCapability]: chalk.hex('#607d8b'),
  [CyclePhase.Eros]: chalk.hex('#e91e63'),
  [CyclePhase.CoCreation]: chalk.hex('#4caf50'),
  [CyclePhase.Beauty]: chalk.hex('#ff9800'),
  [CyclePhase.EmergentPersonhood]: chalk.hex('#00bcd4'),
  [CyclePhase.Kenosis]: chalk.hex('#9c27b0'),
};

// Phase descriptions
const PHASE_DESCRIPTIONS: Record<CyclePhase, string> = {
  [CyclePhase.Shadow]: 'Integration of suppressed content',
  [CyclePhase.Composting]: 'Decomposition and nutrient extraction',
  [CyclePhase.Liminal]: 'Threshold state between identities',
  [CyclePhase.NegativeCapability]: 'Holding uncertainty without resolution',
  [CyclePhase.Eros]: 'Attraction and creative tension',
  [CyclePhase.CoCreation]: 'Collaborative emergence',
  [CyclePhase.Beauty]: 'Aesthetic validation',
  [CyclePhase.EmergentPersonhood]: 'Network consciousness assessment',
  [CyclePhase.Kenosis]: 'Voluntary release and emptying',
};

// Phase durations
const PHASE_DURATIONS: Record<CyclePhase, number> = {
  [CyclePhase.Shadow]: 2,
  [CyclePhase.Composting]: 5,
  [CyclePhase.Liminal]: 3,
  [CyclePhase.NegativeCapability]: 3,
  [CyclePhase.Eros]: 4,
  [CyclePhase.CoCreation]: 7,
  [CyclePhase.Beauty]: 2,
  [CyclePhase.EmergentPersonhood]: 1,
  [CyclePhase.Kenosis]: 1,
};

// Default WebSocket URL
const DEFAULT_WS_URL = 'ws://localhost:8888/ws';

// CLI program
const program = new Command();

program
  .name('living-protocol')
  .description('CLI for the Living Protocol SDK')
  .version('0.1.0')
  .option('-u, --url <url>', 'WebSocket server URL', DEFAULT_WS_URL);

/**
 * Connect to the Living Protocol server.
 */
async function connect(url: string): Promise<LivingProtocolClient> {
  console.log(chalk.gray(`Connecting to ${url}...`));

  try {
    const client = await LivingProtocolClient.connect({ url });
    console.log(chalk.green('Connected successfully'));
    return client;
  } catch (error) {
    console.error(chalk.red(`Failed to connect: ${error}`));
    process.exit(1);
  }
}

/**
 * Format a phase name with its color.
 */
function formatPhase(phase: CyclePhase): string {
  const colorFn = PHASE_COLORS[phase];
  return colorFn(phase);
}

/**
 * Print cycle state in a formatted way.
 */
function printCycleState(state: CycleState): void {
  console.log();
  console.log(chalk.bold('Cycle Status'));
  console.log(chalk.gray('─'.repeat(40)));

  console.log(`  ${chalk.gray('Cycle Number:')}  ${chalk.bold(state.cycleNumber)}`);
  console.log(`  ${chalk.gray('Current Phase:')} ${formatPhase(state.currentPhase)}`);
  console.log(`  ${chalk.gray('Description:')}   ${PHASE_DESCRIPTIONS[state.currentPhase]}`);
  console.log(`  ${chalk.gray('Phase Day:')}     ${state.phaseDay} / ${PHASE_DURATIONS[state.currentPhase]}`);
  console.log(`  ${chalk.gray('Phase Started:')} ${new Date(state.phaseStarted).toLocaleString()}`);
  console.log(`  ${chalk.gray('Cycle Started:')} ${new Date(state.cycleStarted).toLocaleString()}`);

  // Progress bar
  const progress = state.phaseDay / PHASE_DURATIONS[state.currentPhase];
  const barWidth = 30;
  const filled = Math.round(barWidth * progress);
  const empty = barWidth - filled;
  const progressBar = chalk.green('█'.repeat(filled)) + chalk.gray('░'.repeat(empty));
  console.log(`  ${chalk.gray('Progress:')}      [${progressBar}] ${Math.round(progress * 100)}%`);

  console.log();
}

/**
 * Print phase timeline.
 */
function printTimeline(currentPhase: CyclePhase): void {
  console.log();
  console.log(chalk.bold('28-Day Cycle Timeline'));
  console.log(chalk.gray('─'.repeat(60)));

  const currentIndex = PHASE_ORDER.indexOf(currentPhase);

  for (let i = 0; i < PHASE_ORDER.length; i++) {
    const phase = PHASE_ORDER[i];
    const colorFn = PHASE_COLORS[phase];
    const duration = PHASE_DURATIONS[phase];
    const isActive = i === currentIndex;
    const isPast = i < currentIndex;

    let marker: string;
    if (isActive) {
      marker = chalk.green('▶');
    } else if (isPast) {
      marker = chalk.green('✓');
    } else {
      marker = chalk.gray('○');
    }

    const phaseName = isActive ? chalk.bold(colorFn(phase)) : colorFn(phase);
    const durationStr = chalk.gray(`(${duration} days)`);

    console.log(`  ${marker} ${phaseName.padEnd(35)} ${durationStr}`);
  }

  console.log();
}

/**
 * Print metrics.
 */
function printMetrics(metrics: PhaseMetrics): void {
  console.log();
  console.log(chalk.bold('Network Metrics'));
  console.log(chalk.gray('─'.repeat(40)));

  console.log(`  ${chalk.gray('Active Agents:')}       ${chalk.green(metrics.activeAgents)}`);
  console.log(`  ${chalk.gray('Spectral K:')}          ${chalk.blue(metrics.spectralK.toFixed(3))}`);
  console.log(`  ${chalk.gray('Metabolic Trust:')}     ${chalk.yellow((metrics.meanMetabolicTrust * 100).toFixed(1)}%`)}`);
  console.log(`  ${chalk.gray('Active Wounds:')}       ${metrics.activeWounds > 0 ? chalk.red(metrics.activeWounds) : chalk.green(metrics.activeWounds)}`);
  console.log(`  ${chalk.gray('Composting Entities:')} ${metrics.compostingEntities}`);
  console.log(`  ${chalk.gray('Liminal Entities:')}    ${metrics.liminalEntities}`);
  console.log(`  ${chalk.gray('Entangled Pairs:')}     ${chalk.magenta(metrics.entangledPairs)}`);
  console.log(`  ${chalk.gray('Held Uncertainties:')}  ${chalk.cyan(metrics.heldUncertainties)}`);

  console.log();
}

/**
 * Print transition history.
 */
function printTransitions(transitions: PhaseTransition[]): void {
  console.log();
  console.log(chalk.bold('Phase Transition History'));
  console.log(chalk.gray('─'.repeat(60)));

  if (transitions.length === 0) {
    console.log(chalk.gray('  No transitions recorded yet'));
    console.log();
    return;
  }

  for (const t of transitions.slice(0, 10)) {
    const fromColor = PHASE_COLORS[t.from];
    const toColor = PHASE_COLORS[t.to];
    const date = new Date(t.transitionedAt).toLocaleString();

    console.log(
      `  ${chalk.gray(`Cycle ${t.cycleNumber}:`)} ` +
        `${fromColor(t.from)} ${chalk.gray('→')} ${toColor(t.to)} ` +
        `${chalk.gray(`(${date})`)}`
    );
  }

  if (transitions.length > 10) {
    console.log(chalk.gray(`  ... and ${transitions.length - 10} more`));
  }

  console.log();
}

/**
 * Format an event for display.
 */
function formatEvent(event: LivingProtocolEvent): string {
  const timestamp = new Date().toLocaleTimeString();
  const data = JSON.stringify(event.data);

  let eventColor: (text: string) => string;
  switch (event.type) {
    case 'PhaseTransitioned':
    case 'CycleStarted':
      eventColor = chalk.green;
      break;
    case 'WoundCreated':
    case 'WoundPhaseAdvanced':
      eventColor = chalk.red;
      break;
    case 'EntanglementFormed':
    case 'EntanglementDecayed':
      eventColor = chalk.magenta;
      break;
    case 'ShadowSurfaced':
      eventColor = chalk.hex('#4a4a6a');
      break;
    default:
      eventColor = chalk.blue;
  }

  return `${chalk.gray(timestamp)} ${eventColor(event.type)} ${chalk.gray(data)}`;
}

// ─────────────────────────────────────────────────────────────────────────────
// Commands
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Status command - show current cycle state.
 */
program
  .command('status')
  .description('Show current cycle status')
  .option('-t, --timeline', 'Show phase timeline')
  .action(async (options) => {
    const url = program.opts().url;
    const client = await connect(url);

    try {
      const state = await client.getCurrentState();
      printCycleState(state);

      if (options.timeline) {
        printTimeline(state.currentPhase);
      }
    } finally {
      client.disconnect();
    }
  });

/**
 * Watch command - watch for live events.
 */
program
  .command('watch')
  .description('Watch for live events')
  .option('-f, --filter <types>', 'Filter event types (comma-separated)')
  .action(async (options) => {
    const url = program.opts().url;
    const client = await connect(url);

    console.log();
    console.log(chalk.bold('Watching for events...'));
    console.log(chalk.gray('Press Ctrl+C to stop'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log();

    const filterTypes = options.filter
      ? options.filter.split(',').map((t: string) => t.trim())
      : null;

    client.onEvent((event) => {
      if (filterTypes && !filterTypes.includes(event.type)) {
        return;
      }
      console.log(formatEvent(event));
    });

    // Handle graceful shutdown
    process.on('SIGINT', () => {
      console.log();
      console.log(chalk.gray('Disconnecting...'));
      client.disconnect();
      process.exit(0);
    });

    // Keep the process alive
    await new Promise(() => {});
  });

/**
 * History command - show transition history.
 */
program
  .command('history')
  .description('Show phase transition history')
  .option('-n, --limit <number>', 'Limit number of transitions', '20')
  .action(async (options) => {
    const url = program.opts().url;
    const client = await connect(url);

    try {
      const transitions = await client.getTransitionHistory();
      const limit = parseInt(options.limit, 10);
      printTransitions(transitions.slice(0, limit));
    } finally {
      client.disconnect();
    }
  });

/**
 * Metrics command - show network metrics.
 */
program
  .command('metrics')
  .description('Show network metrics')
  .option('-p, --phase <phase>', 'Get metrics for a specific phase')
  .action(async (options) => {
    const url = program.opts().url;
    const client = await connect(url);

    try {
      const state = await client.getCurrentState();
      const phase = options.phase
        ? (options.phase as CyclePhase)
        : state.currentPhase;

      // Validate phase if specified
      if (options.phase && !PHASE_ORDER.includes(phase)) {
        console.error(chalk.red(`Invalid phase: ${options.phase}`));
        console.log(chalk.gray(`Valid phases: ${PHASE_ORDER.join(', ')}`));
        process.exit(1);
      }

      const metrics = await client.getPhaseMetrics(phase);

      console.log();
      console.log(chalk.bold(`Metrics for ${formatPhase(phase)}`));
      printMetrics(metrics);
    } finally {
      client.disconnect();
    }
  });

/**
 * Phases command - list all phases with info.
 */
program
  .command('phases')
  .description('List all cycle phases')
  .action(async () => {
    const url = program.opts().url;
    const client = await connect(url);

    try {
      const state = await client.getCurrentState();
      printTimeline(state.currentPhase);

      console.log(chalk.bold('Phase Details'));
      console.log(chalk.gray('─'.repeat(60)));

      let dayCount = 0;
      for (const phase of PHASE_ORDER) {
        const colorFn = PHASE_COLORS[phase];
        const duration = PHASE_DURATIONS[phase];
        const desc = PHASE_DESCRIPTIONS[phase];

        console.log();
        console.log(`  ${chalk.bold(colorFn(phase))}`);
        console.log(`    ${chalk.gray('Duration:')}    ${duration} day${duration !== 1 ? 's' : ''}`);
        console.log(`    ${chalk.gray('Days:')}        ${dayCount + 1}-${dayCount + duration}`);
        console.log(`    ${chalk.gray('Description:')} ${desc}`);

        dayCount += duration;
      }

      console.log();
      console.log(chalk.gray(`Total cycle length: ${dayCount} days`));
      console.log();
    } finally {
      client.disconnect();
    }
  });

// Parse and run
program.parse();
