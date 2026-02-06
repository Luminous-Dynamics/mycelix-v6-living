---
sidebar_position: 3
title: Quick Start
---

# Quick Start: 5-Minute Tutorial

Build a living application that responds to the 28-day cycle.

## What We're Building

A simple "mood tracker" that adapts its behavior based on the current phase:
- **Dawn**: Gentle, encouraging prompts
- **Surge**: High-energy, productive focus
- **Settle**: Reflective, pattern analysis
- **Rest**: Minimal interaction, self-care reminders

## Step 1: Initialize Project

```bash
mkdir mycelix-mood && cd mycelix-mood
npm init -y
npm install @mycelix/core @mycelix/server
```

## Step 2: Create the Server

```typescript
// server.ts
import { Mycelix, Phase } from '@mycelix/core';
import { createServer } from '@mycelix/server';

const mycelix = new Mycelix({
  cycle: { startDate: new Date().toISOString().split('T')[0] }
});

const server = createServer(mycelix, { port: 8080 });

// Define phase-aware behavior
mycelix.on('phaseChange', (phase: Phase) => {
  console.log(`Entering ${phase} phase`);
});

// Start the server
server.start();
console.log('Mycelix server running on http://localhost:8080');
```

## Step 3: Add a Living Primitive

Create a `Pulse` that emits mood check-ins:

```typescript
// mood-pulse.ts
import { Mycelix, Pulse } from '@mycelix/core';

const mycelix = new Mycelix();

// Create a pulse that adapts to the current phase
const moodPulse = new Pulse({
  name: 'mood-checkin',

  // Interval varies by phase
  interval: (phase) => ({
    Dawn: '4h',    // Gentle, every 4 hours
    Surge: '1h',   // Frequent during peak
    Settle: '2h',  // Moderate
    Rest: '8h',    // Minimal
  }[phase]),

  // The pulse action
  emit: async (context) => {
    const prompts = {
      Dawn: "Good morning! How are you feeling as you start your day?",
      Surge: "You're in the zone! Quick check: energy level 1-10?",
      Settle: "Time to reflect. What patterns do you notice today?",
      Rest: "Be gentle with yourself. Need anything?",
    };

    return {
      prompt: prompts[context.phase],
      timestamp: context.now,
      cycleDay: context.cycleDay,
    };
  },
});

mycelix.register(moodPulse);
```

## Step 4: Create a WebSocket Handler

```typescript
// ws-handler.ts
import { Mycelix, Thread } from '@mycelix/core';

const mycelix = new Mycelix();

// Create a thread to handle mood submissions
const moodThread = new Thread({
  name: 'mood-handler',

  handle: async (message, context) => {
    const { mood, energy, note } = message;

    // Store with phase context
    await context.store.put('moods', {
      mood,
      energy,
      note,
      phase: context.phase,
      cycleDay: context.cycleDay,
      timestamp: context.now,
    });

    // Phase-specific response
    const responses = {
      Dawn: `Great start! Your ${mood} mood sets the tone.`,
      Surge: `Logged! Keep that ${energy}/10 energy flowing.`,
      Settle: `Noted. I see patterns forming...`,
      Rest: `Rest well. ${mood} is perfectly valid.`,
    };

    return { response: responses[context.phase] };
  },
});

mycelix.register(moodThread);
```

## Step 5: Run Your Application

```bash
npx tsx server.ts
```

## Step 6: Connect a Client

```typescript
// client.ts
import { MycelixClient } from '@mycelix/sdk';

const client = new MycelixClient('ws://localhost:9090');

// Listen for mood prompts
client.on('mood-checkin', (data) => {
  console.log(data.prompt);
});

// Submit a mood
await client.send('mood-handler', {
  mood: 'focused',
  energy: 8,
  note: 'Deep work session going well',
});
```

## Complete Example

Here's everything in one file:

```typescript
// app.ts
import { Mycelix, Pulse, Thread, createServer } from '@mycelix/core';

const mycelix = new Mycelix();

// Phase-aware pulse
const moodPulse = new Pulse({
  name: 'mood-checkin',
  interval: (phase) => ({ Dawn: '4h', Surge: '1h', Settle: '2h', Rest: '8h' }[phase]),
  emit: async ({ phase, cycleDay }) => ({
    prompt: `Day ${cycleDay} (${phase}): How are you?`,
  }),
});

// Mood handler thread
const moodThread = new Thread({
  name: 'mood-handler',
  handle: async (msg, ctx) => {
    await ctx.store.put('moods', { ...msg, phase: ctx.phase });
    return { saved: true };
  },
});

// Register and start
mycelix.register(moodPulse, moodThread);
createServer(mycelix, { port: 8080 }).start();

console.log('Living mood tracker running!');
console.log(`Current phase: ${mycelix.currentPhase}`);
console.log(`Cycle day: ${mycelix.cycleDay}/28`);
```

## What's Next?

- [The 28-Day Cycle](./concepts/cycle) - Understand the rhythm
- [All 21 Primitives](./concepts/primitives) - Explore building blocks
- [TypeScript SDK](./sdks/typescript) - Full SDK documentation
