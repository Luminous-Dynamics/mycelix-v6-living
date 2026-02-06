# Cycle Engine Plugin Development Guide

This guide covers how to create, configure, and deploy plugins for the Mycelix Metabolism Cycle Engine.

## Overview

The plugin architecture allows you to extend the cycle engine's functionality without modifying core code. Plugins can:

- React to phase transitions and lifecycle events
- Collect and export metrics
- Send notifications to external systems
- Implement custom business logic
- Integrate with external services

## Table of Contents

1. [Quick Start](#quick-start)
2. [Plugin Trait API](#plugin-trait-api)
3. [Plugin Context](#plugin-context)
4. [Plugin Configuration](#plugin-configuration)
5. [Plugin Priority](#plugin-priority)
6. [Built-in Plugins](#built-in-plugins)
7. [Middleware Architecture](#middleware-architecture)
8. [Dynamic Plugin Loading](#dynamic-plugin-loading)
9. [Plugin Manifest Format](#plugin-manifest-format)
10. [Example Plugin Walkthrough](#example-plugin-walkthrough)
11. [Best Practices](#best-practices)

## Quick Start

### Creating a Simple Plugin

```rust
use std::any::Any;
use cycle_engine::plugin::{Plugin, PluginConfig, PluginContext, PluginError, PluginPriority};
use living_core::{CyclePhase, LivingProtocolEvent};

pub struct MyPlugin {
    name: String,
}

impl MyPlugin {
    pub fn new() -> Self {
        Self {
            name: "my-plugin".to_string(),
        }
    }
}

impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn on_phase_enter(&self, phase: CyclePhase, ctx: &mut PluginContext) {
        println!("Entering phase: {:?}", phase);
    }

    fn on_event(&self, event: &LivingProtocolEvent, ctx: &mut PluginContext) {
        println!("Event received: {:?}", std::mem::discriminant(event));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
```

### Registering the Plugin

```rust
use cycle_engine::{CycleEngineBuilder, plugin::PluginConfig};

let engine = CycleEngineBuilder::new()
    .with_plugin(Box::new(MyPlugin::new()), PluginConfig::enabled())
    .build();
```

## Plugin Trait API

The `Plugin` trait defines the interface all plugins must implement:

```rust
pub trait Plugin: Send + Sync {
    // Required methods
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    // Lifecycle hooks (optional)
    fn on_load(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError>;
    fn on_unload(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError>;

    // Phase hooks (optional)
    fn on_phase_enter(&self, phase: CyclePhase, ctx: &mut PluginContext);
    fn on_phase_exit(&self, phase: CyclePhase, ctx: &mut PluginContext);
    fn on_phase_transition(&self, from: CyclePhase, to: CyclePhase, ctx: &mut PluginContext);

    // Tick hook (optional, called frequently)
    fn on_tick(&self, ctx: &mut PluginContext);

    // Event hook (optional)
    fn on_event(&self, event: &LivingProtocolEvent, ctx: &mut PluginContext);

    // Cycle hooks (optional)
    fn on_cycle_start(&self, cycle_number: u64, ctx: &mut PluginContext);
    fn on_cycle_complete(&self, cycle_number: u64, ctx: &mut PluginContext);

    // Configuration (optional)
    fn priority(&self) -> PluginPriority;
    fn configure(&mut self, config: PluginConfig) -> Result<(), PluginError>;
    fn config_schema(&self) -> Option<serde_json::Value>;
    fn status(&self) -> PluginStatus;
    fn metrics(&self) -> serde_json::Value;
}
```

### Hook Execution Order

1. `on_load` - Called once when the plugin is loaded
2. `on_cycle_start` - Called when a new cycle begins
3. `on_phase_enter` - Called when entering a phase
4. `on_tick` - Called on each engine tick (frequently)
5. `on_event` - Called for each Living Protocol event
6. `on_phase_exit` - Called when exiting a phase
7. `on_phase_transition` - Called during phase transitions
8. `on_cycle_complete` - Called when a cycle finishes
9. `on_unload` - Called once when the plugin is unloaded

## Plugin Context

The `PluginContext` provides access to engine state and allows plugins to interact with the system:

```rust
impl PluginContext {
    // Read-only access to state
    pub fn cycle_state(&self) -> &CycleState;
    pub fn current_phase(&self) -> CyclePhase;
    pub fn cycle_number(&self) -> u64;
    pub fn phase_day(&self) -> u32;
    pub fn metrics(&self) -> &PhaseMetrics;

    // Plugin metadata storage
    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value);
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value>;
    pub fn remove_metadata(&mut self, key: &str) -> Option<serde_json::Value>;

    // Event emission
    pub fn emit_event(&mut self, event: LivingProtocolEvent);

    // Logging
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>);

    // Shared state between plugins
    pub fn set_shared_state(&mut self, plugin_name: &str, value: serde_json::Value);
    pub fn get_shared_state(&self, plugin_name: &str) -> Option<&serde_json::Value>;

    // Flow control
    pub fn request_pause(&mut self);
}
```

### Emitting Events

Plugins can emit Living Protocol events:

```rust
fn on_tick(&self, ctx: &mut PluginContext) {
    // Emit a custom event
    ctx.emit_event(LivingProtocolEvent::CycleStarted(CycleStartedEvent {
        cycle_number: ctx.cycle_number(),
        started_at: chrono::Utc::now(),
    }));
}
```

### Logging

Use the context's logging facilities:

```rust
fn on_phase_enter(&self, phase: CyclePhase, ctx: &mut PluginContext) {
    ctx.log(LogLevel::Info, format!("Entering phase {:?}", phase));
    ctx.log(LogLevel::Debug, "Phase metrics captured");
    ctx.log(LogLevel::Warn, "Unusual pattern detected");
}
```

## Plugin Configuration

Plugins can be configured using `PluginConfig`:

```rust
use cycle_engine::plugin::{PluginConfig, PluginPriority};

// Simple enabled configuration
let config = PluginConfig::enabled();

// With settings
let config = PluginConfig::enabled()
    .with_settings(serde_json::json!({
        "log_level": "debug",
        "threshold": 0.5,
        "enabled_features": ["feature1", "feature2"]
    }));

// With priority override
let config = PluginConfig::enabled()
    .with_priority(PluginPriority::High);

// Disabled plugin
let config = PluginConfig::disabled();
```

### Reading Configuration in Plugins

```rust
fn configure(&mut self, config: PluginConfig) -> Result<(), PluginError> {
    // Type-safe access to settings
    if let Some(threshold) = config.get_setting::<f64>("threshold") {
        self.threshold = threshold;
    }

    if let Some(log_level) = config.get_setting::<String>("log_level") {
        self.log_level = log_level;
    }

    Ok(())
}
```

### Configuration Schema

Plugins can provide a JSON Schema for their configuration:

```rust
fn config_schema(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "type": "object",
        "properties": {
            "threshold": {
                "type": "number",
                "minimum": 0.0,
                "maximum": 1.0,
                "default": 0.5
            },
            "log_level": {
                "type": "string",
                "enum": ["trace", "debug", "info", "warn", "error"],
                "default": "info"
            }
        }
    }))
}
```

## Plugin Priority

Priority determines the order in which plugins receive hook calls:

```rust
pub enum PluginPriority {
    System = 0,    // Called first (0-99)
    High = 100,    // Called early (100-199)
    Normal = 200,  // Default (200-299)
    Low = 300,     // Called later (300-399)
    Lowest = 400,  // Called last (400+)
}
```

Within the same priority level, plugins are called in load order.

### Recommended Priority Usage

- **System**: Critical infrastructure plugins (security, rate limiting)
- **High**: Logging, tracing, early validation
- **Normal**: Business logic plugins
- **Low**: Metrics collection, monitoring
- **Lowest**: Cleanup, final notifications

## Built-in Plugins

### Logging Plugin

Detailed phase transition and event logging.

```rust
use cycle_plugin_logging::LoggingPlugin;

let config = PluginConfig::enabled().with_settings(serde_json::json!({
    "log_level": "info",
    "log_metrics": true,
    "log_events": true,
    "output_format": "json"
}));

builder.with_plugin(Box::new(LoggingPlugin::new()), config)
```

### Metrics Export Plugin

Export metrics to Prometheus, StatsD, or JSON.

```rust
use cycle_plugin_metrics_export::MetricsExportPlugin;

let config = PluginConfig::enabled().with_settings(serde_json::json!({
    "export_format": "prometheus",
    "metric_prefix": "mycelix_cycle",
    "include_phase_metrics": true,
    "labels": {
        "environment": "production",
        "cluster": "main"
    }
}));

builder.with_plugin(Box::new(MetricsExportPlugin::new()), config)
```

### Notification Plugin

Send notifications on important events.

```rust
use cycle_plugin_notification::NotificationPlugin;

let config = PluginConfig::enabled().with_settings(serde_json::json!({
    "channels": ["webhook", "slack"],
    "webhook_url": "https://example.com/webhook",
    "notify_on_phase_transition": true,
    "notify_on_wounds": true,
    "min_wound_severity": "moderate"
}));

builder.with_plugin(Box::new(NotificationPlugin::new()), config)
```

## Middleware Architecture

Middleware intercepts RPC requests and responses:

```rust
use cycle_engine::middleware::{
    Middleware, MiddlewareChain, MiddlewareNext, MiddlewareResult,
    RpcRequest, RpcResponse,
    LoggingMiddleware, MetricsMiddleware, RateLimitMiddleware,
};

// Built-in middlewares
let mut chain = MiddlewareChain::new();
chain.add(LoggingMiddleware::new().with_params().with_results());
chain.add(MetricsMiddleware::new());
chain.add(RateLimitMiddleware::new(100, Duration::from_secs(60)));

// Process a request
let response = chain.process(request, &handler_fn)?;
```

### Custom Middleware

```rust
pub struct AuthMiddleware {
    secret_key: String,
}

impl Middleware for AuthMiddleware {
    fn name(&self) -> &str {
        "auth"
    }

    fn handle_request(
        &self,
        req: RpcRequest,
        next: MiddlewareNext<'_>,
    ) -> MiddlewareResult {
        // Validate auth token
        match req.get_metadata("authorization") {
            Some(token) if self.validate_token(token) => {
                // Continue to next middleware
                next.run(req)
            }
            _ => {
                // Reject unauthorized requests
                Ok(RpcResponse::error(
                    req.id,
                    RpcError::new(RpcError::UNAUTHORIZED, "Invalid or missing token"),
                ))
            }
        }
    }
}
```

## Dynamic Plugin Loading

Enable the `dynamic-plugins` feature for shared library loading:

```toml
[dependencies]
cycle-engine = { path = "crates/cycle-engine", features = ["dynamic-plugins"] }
```

### Loading a Dynamic Plugin

```rust
use cycle_engine::plugin::dynamic::{DynamicPluginLoader, DynamicPlugin};

let loader = DynamicPluginLoader::new(vec![
    PathBuf::from("./plugins"),
    PathBuf::from("/usr/lib/mycelix/plugins"),
]);

// Discover plugins
let manifests = loader.discover_plugins();

// Load from manifest
unsafe {
    let (manifest, plugin) = loader.load_from_manifest(&manifest_path)?;
    manager.load_plugin(plugin.into_plugin(), manifest.config)?;
}
```

### Creating a Dynamic Plugin

Your plugin library must export a `create_plugin` function:

```rust
// In your plugin's lib.rs
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    let plugin = Box::new(MyPlugin::new());
    Box::into_raw(plugin)
}
```

Compile as a cdylib:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

## Plugin Manifest Format

Dynamic plugins require a `plugin.toml` manifest:

```toml
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "A custom plugin for the cycle engine"
authors = ["Your Name <you@example.com>"]
license = "MIT"
min_engine_version = "0.1.0"
library = "my_plugin"  # Without platform extension
entry_point = "create_plugin"  # Optional, default: "create_plugin"

[[dependencies]]
name = "logging"
version = ">=0.1.0"
optional = false

[[dependencies]]
name = "metrics-export"
version = ">=0.1.0"
optional = true

[config]
enabled = true

[config.settings]
# Plugin-specific settings
threshold = 0.5
log_level = "info"
```

## Example Plugin Walkthrough

Let's create a complete plugin that tracks cycle health:

```rust
use std::any::Any;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use cycle_engine::plugin::{
    LogLevel, Plugin, PluginConfig, PluginContext, PluginError,
    PluginPriority, PluginStatus,
};
use living_core::{CyclePhase, LivingProtocolEvent};

/// Configuration for the health monitor plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitorConfig {
    /// Minimum acceptable metabolic trust
    pub min_trust_threshold: f64,
    /// Maximum acceptable active wounds
    pub max_wounds_threshold: u64,
    /// Alert on threshold breach
    pub alert_on_breach: bool,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            min_trust_threshold: 0.5,
            max_wounds_threshold: 10,
            alert_on_breach: true,
        }
    }
}

/// Health status snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct HealthSnapshot {
    pub timestamp: DateTime<Utc>,
    pub cycle_number: u64,
    pub phase: String,
    pub metabolic_trust: f64,
    pub active_wounds: u64,
    pub is_healthy: bool,
}

/// Plugin that monitors cycle health.
pub struct HealthMonitorPlugin {
    config: HealthMonitorConfig,
    snapshots: VecDeque<HealthSnapshot>,
    breach_count: AtomicU64,
    max_snapshots: usize,
}

impl HealthMonitorPlugin {
    pub fn new() -> Self {
        Self {
            config: HealthMonitorConfig::default(),
            snapshots: VecDeque::new(),
            breach_count: AtomicU64::new(0),
            max_snapshots: 1000,
        }
    }

    fn check_health(&self, ctx: &PluginContext) -> HealthSnapshot {
        let metrics = ctx.metrics();
        let is_healthy = metrics.mean_metabolic_trust >= self.config.min_trust_threshold
            && metrics.active_wounds <= self.config.max_wounds_threshold;

        HealthSnapshot {
            timestamp: Utc::now(),
            cycle_number: ctx.cycle_number(),
            phase: format!("{:?}", ctx.current_phase()),
            metabolic_trust: metrics.mean_metabolic_trust,
            active_wounds: metrics.active_wounds,
            is_healthy,
        }
    }

    pub fn recent_snapshots(&self) -> &VecDeque<HealthSnapshot> {
        &self.snapshots
    }

    pub fn breach_count(&self) -> u64 {
        self.breach_count.load(Ordering::Relaxed)
    }
}

impl Plugin for HealthMonitorPlugin {
    fn name(&self) -> &str {
        "health-monitor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn priority(&self) -> PluginPriority {
        PluginPriority::Low
    }

    fn on_load(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError> {
        ctx.log(LogLevel::Info, "Health monitor plugin loaded");
        Ok(())
    }

    fn on_tick(&self, ctx: &mut PluginContext) {
        let snapshot = self.check_health(ctx);

        if !snapshot.is_healthy {
            self.breach_count.fetch_add(1, Ordering::Relaxed);

            if self.config.alert_on_breach {
                ctx.log(
                    LogLevel::Warn,
                    format!(
                        "Health threshold breach: trust={:.2}, wounds={}",
                        snapshot.metabolic_trust, snapshot.active_wounds
                    ),
                );
            }
        }

        // Store snapshot in shared state for other plugins
        ctx.set_shared_state(
            self.name(),
            serde_json::to_value(&snapshot).unwrap_or_default(),
        );
    }

    fn on_phase_transition(
        &self,
        from: CyclePhase,
        to: CyclePhase,
        ctx: &mut PluginContext,
    ) {
        let snapshot = self.check_health(ctx);

        ctx.log(
            LogLevel::Info,
            format!(
                "Health at transition {:?}->{:?}: healthy={}",
                from, to, snapshot.is_healthy
            ),
        );
    }

    fn configure(&mut self, config: PluginConfig) -> Result<(), PluginError> {
        if let Ok(health_config) =
            serde_json::from_value::<HealthMonitorConfig>(config.settings)
        {
            self.config = health_config;
        }
        Ok(())
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "min_trust_threshold": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "default": 0.5
                },
                "max_wounds_threshold": {
                    "type": "integer",
                    "minimum": 0,
                    "default": 10
                },
                "alert_on_breach": {
                    "type": "boolean",
                    "default": true
                }
            }
        }))
    }

    fn status(&self) -> PluginStatus {
        PluginStatus::Running
    }

    fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "breach_count": self.breach_count(),
            "snapshots_collected": self.snapshots.len(),
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Entry point for dynamic loading
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    Box::into_raw(Box::new(HealthMonitorPlugin::new()))
}
```

## Best Practices

### 1. Keep Hooks Lightweight

The `on_tick` hook is called frequently. Avoid expensive operations:

```rust
// Bad: Heavy computation on every tick
fn on_tick(&self, ctx: &mut PluginContext) {
    let result = expensive_computation();  // Don't do this
    ctx.set_metadata("result", result);
}

// Good: Use sampling or batching
fn on_tick(&self, ctx: &mut PluginContext) {
    self.tick_count.fetch_add(1, Ordering::Relaxed);

    // Only process every 100 ticks
    if self.tick_count.load(Ordering::Relaxed) % 100 == 0 {
        let result = expensive_computation();
        ctx.set_metadata("result", result);
    }
}
```

### 2. Handle Errors Gracefully

Don't let plugin errors crash the engine:

```rust
fn on_event(&self, event: &LivingProtocolEvent, ctx: &mut PluginContext) {
    if let Err(e) = self.process_event(event) {
        ctx.log(LogLevel::Error, format!("Failed to process event: {}", e));
        // Continue operation, don't panic
    }
}
```

### 3. Use Appropriate Log Levels

```rust
ctx.log(LogLevel::Trace, "Detailed debug info");
ctx.log(LogLevel::Debug, "Development debugging");
ctx.log(LogLevel::Info, "Normal operation info");
ctx.log(LogLevel::Warn, "Unexpected but recoverable");
ctx.log(LogLevel::Error, "Operation failed");
```

### 4. Document Configuration

Always provide a config schema and document all options:

```rust
fn config_schema(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "type": "object",
        "description": "Health monitor plugin configuration",
        "properties": {
            "threshold": {
                "type": "number",
                "description": "Minimum acceptable value (0.0-1.0)",
                "default": 0.5
            }
        },
        "required": ["threshold"]
    }))
}
```

### 5. Support Both Static and Dynamic Loading

Make your plugin work in both scenarios:

```rust
// For static linking
impl Default for MyPlugin {
    fn default() -> Self {
        Self::new()
    }
}

// For dynamic loading
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn Plugin {
    Box::into_raw(Box::new(MyPlugin::new()))
}
```

### 6. Version Your Plugin

Follow semantic versioning and check engine compatibility:

```rust
fn version(&self) -> &str {
    env!("CARGO_PKG_VERSION")
}

fn on_load(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError> {
    // Check engine version if needed
    if !self.is_compatible_engine() {
        return Err(PluginError::VersionMismatch {
            expected: ">=0.1.0".to_string(),
            actual: "unknown".to_string(),
        });
    }
    Ok(())
}
```

### 7. Clean Up Resources

Always implement proper cleanup:

```rust
fn on_unload(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError> {
    // Flush any pending data
    self.flush_buffer()?;

    // Close connections
    self.close_connections()?;

    ctx.log(LogLevel::Info, "Plugin unloaded cleanly");
    Ok(())
}
```

## API Reference

For complete API documentation, run:

```bash
cargo doc --package cycle-engine --open
```

## Support

For questions or issues:
- Open an issue on GitHub
- Join the Mycelix Discord community
- Check the examples in `plugins/` directory
