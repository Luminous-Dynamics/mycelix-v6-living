//! Plugin architecture for the Metabolism Cycle Engine.
//!
//! Plugins can hook into the cycle engine lifecycle to extend functionality
//! without modifying core engine code.
//!
//! # Example
//!
//! ```rust,ignore
//! use cycle_engine::plugin::{Plugin, PluginContext, PluginPriority};
//! use living_core::{CyclePhase, LivingProtocolEvent};
//!
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn name(&self) -> &str { "my-plugin" }
//!     fn version(&self) -> &str { "1.0.0" }
//!
//!     fn on_phase_enter(&self, phase: CyclePhase, ctx: &mut PluginContext) {
//!         println!("Entering phase: {:?}", phase);
//!     }
//! }
//! ```

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use living_core::{CyclePhase, CycleState, LivingProtocolEvent, PhaseMetrics};

// =============================================================================
// Plugin Trait
// =============================================================================

/// Priority determines the order in which plugins are called.
/// Lower values are called first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PluginPriority {
    /// Called first, for critical system plugins (0-99)
    System = 0,
    /// Called early, for infrastructure plugins (100-199)
    High = 100,
    /// Default priority for most plugins (200-299)
    Normal = 200,
    /// Called later, for monitoring/logging plugins (300-399)
    Low = 300,
    /// Called last, for cleanup plugins (400+)
    Lowest = 400,
}

impl Default for PluginPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// The main plugin trait. Implement this to create a cycle engine plugin.
///
/// All methods have default no-op implementations, so you only need to
/// implement the hooks you care about.
pub trait Plugin: Send + Sync {
    /// Unique name of the plugin.
    fn name(&self) -> &str;

    /// Version of the plugin (semver recommended).
    fn version(&self) -> &str;

    /// Plugin priority for ordering hook execution.
    fn priority(&self) -> PluginPriority {
        PluginPriority::Normal
    }

    /// Called when the plugin is loaded into the engine.
    ///
    /// Use this for initialization that requires access to the engine context.
    fn on_load(&mut self, _ctx: &mut PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Called when the plugin is unloaded from the engine.
    fn on_unload(&mut self, _ctx: &mut PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Called when the engine enters a new phase.
    fn on_phase_enter(&self, _phase: CyclePhase, _ctx: &mut PluginContext) {}

    /// Called when the engine exits a phase.
    fn on_phase_exit(&self, _phase: CyclePhase, _ctx: &mut PluginContext) {}

    /// Called on each engine tick.
    ///
    /// Note: This is called frequently, so keep implementations lightweight.
    fn on_tick(&self, _ctx: &mut PluginContext) {}

    /// Called when a Living Protocol event is emitted.
    fn on_event(&self, _event: &LivingProtocolEvent, _ctx: &mut PluginContext) {}

    /// Called when a phase transition occurs.
    fn on_phase_transition(
        &self,
        _from: CyclePhase,
        _to: CyclePhase,
        _ctx: &mut PluginContext,
    ) {
    }

    /// Called when a new cycle starts.
    fn on_cycle_start(&self, _cycle_number: u64, _ctx: &mut PluginContext) {}

    /// Called when a cycle completes.
    fn on_cycle_complete(&self, _cycle_number: u64, _ctx: &mut PluginContext) {}

    /// Get plugin-specific configuration schema (JSON Schema format).
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// Configure the plugin with the given configuration.
    fn configure(&mut self, _config: PluginConfig) -> Result<(), PluginError> {
        Ok(())
    }

    /// Get the plugin's current status.
    fn status(&self) -> PluginStatus {
        PluginStatus::Running
    }

    /// Get plugin-specific metrics.
    fn metrics(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    /// Cast to Any for downcasting (required for dynamic plugin access).
    fn as_any(&self) -> &dyn Any;

    /// Cast to Any for mutable downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Status of a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginStatus {
    /// Plugin is loaded but not yet initialized
    Loaded,
    /// Plugin is running normally
    Running,
    /// Plugin is paused
    Paused,
    /// Plugin encountered an error
    Error,
    /// Plugin is being unloaded
    Unloading,
}

// =============================================================================
// Plugin Context
// =============================================================================

/// Context provided to plugins during lifecycle hooks.
///
/// Provides read-only access to engine state and metrics, plus a mutable
/// metadata store for plugin-specific data.
pub struct PluginContext {
    /// Current cycle state (read-only)
    cycle_state: CycleState,
    /// Phase metrics (read-only)
    phase_metrics: PhaseMetrics,
    /// Plugin-specific metadata storage
    metadata: HashMap<String, serde_json::Value>,
    /// Events emitted by the plugin during this hook
    emitted_events: Vec<LivingProtocolEvent>,
    /// Log messages from the plugin
    log_messages: Vec<PluginLogMessage>,
    /// Whether the plugin requested a pause
    pause_requested: bool,
    /// Shared state between plugins (keyed by plugin name)
    shared_state: HashMap<String, serde_json::Value>,
}

impl PluginContext {
    /// Create a new plugin context.
    pub fn new(cycle_state: CycleState, phase_metrics: PhaseMetrics) -> Self {
        Self {
            cycle_state,
            phase_metrics,
            metadata: HashMap::new(),
            emitted_events: Vec::new(),
            log_messages: Vec::new(),
            pause_requested: false,
            shared_state: HashMap::new(),
        }
    }

    /// Get the current cycle state.
    pub fn cycle_state(&self) -> &CycleState {
        &self.cycle_state
    }

    /// Get the current phase.
    pub fn current_phase(&self) -> CyclePhase {
        self.cycle_state.current_phase
    }

    /// Get the current cycle number.
    pub fn cycle_number(&self) -> u64 {
        self.cycle_state.cycle_number
    }

    /// Get the current phase day.
    pub fn phase_day(&self) -> u32 {
        self.cycle_state.phase_day
    }

    /// Get phase metrics.
    pub fn metrics(&self) -> &PhaseMetrics {
        &self.phase_metrics
    }

    /// Store plugin-specific metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }

    /// Get plugin-specific metadata.
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// Remove plugin-specific metadata.
    pub fn remove_metadata(&mut self, key: &str) -> Option<serde_json::Value> {
        self.metadata.remove(key)
    }

    /// Emit an event from the plugin.
    pub fn emit_event(&mut self, event: LivingProtocolEvent) {
        self.emitted_events.push(event);
    }

    /// Get events emitted during this hook call.
    pub fn take_emitted_events(&mut self) -> Vec<LivingProtocolEvent> {
        std::mem::take(&mut self.emitted_events)
    }

    /// Log a message from the plugin.
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.log_messages.push(PluginLogMessage {
            level,
            message: message.into(),
            timestamp: chrono::Utc::now(),
        });
    }

    /// Get log messages from this hook call.
    pub fn take_log_messages(&mut self) -> Vec<PluginLogMessage> {
        std::mem::take(&mut self.log_messages)
    }

    /// Request the engine to pause after this hook completes.
    pub fn request_pause(&mut self) {
        self.pause_requested = true;
    }

    /// Check if a pause was requested.
    pub fn is_pause_requested(&self) -> bool {
        self.pause_requested
    }

    /// Set shared state accessible by other plugins.
    pub fn set_shared_state(&mut self, plugin_name: &str, value: serde_json::Value) {
        self.shared_state.insert(plugin_name.to_string(), value);
    }

    /// Get shared state from another plugin.
    pub fn get_shared_state(&self, plugin_name: &str) -> Option<&serde_json::Value> {
        self.shared_state.get(plugin_name)
    }

    /// Update the cycle state (internal use only).
    pub(crate) fn update_cycle_state(&mut self, state: CycleState) {
        self.cycle_state = state;
    }

    /// Update the phase metrics (internal use only).
    pub(crate) fn update_phase_metrics(&mut self, metrics: PhaseMetrics) {
        self.phase_metrics = metrics;
    }
}

/// Log level for plugin messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// A log message from a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginLogMessage {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// =============================================================================
// Plugin Configuration
// =============================================================================

/// Configuration for a plugin.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Whether the plugin is enabled.
    pub enabled: bool,
    /// Plugin-specific configuration (arbitrary JSON).
    #[serde(default)]
    pub settings: serde_json::Value,
    /// Priority override (if None, uses plugin's default).
    pub priority: Option<PluginPriority>,
}

impl PluginConfig {
    /// Create a new enabled plugin config with default settings.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            settings: serde_json::Value::Null,
            priority: None,
        }
    }

    /// Create a new disabled plugin config.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            settings: serde_json::Value::Null,
            priority: None,
        }
    }

    /// Set the plugin settings.
    pub fn with_settings(mut self, settings: serde_json::Value) -> Self {
        self.settings = settings;
        self
    }

    /// Set the priority override.
    pub fn with_priority(mut self, priority: PluginPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Get a typed setting value.
    pub fn get_setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.settings
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

// =============================================================================
// Plugin Error
// =============================================================================

/// Errors that can occur in plugin operations.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin configuration error: {0}")]
    ConfigurationError(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Plugin hook error in {plugin}: {message}")]
    HookError { plugin: String, message: String },

    #[error("Plugin version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    #[error("Dynamic loading error: {0}")]
    DynamicLoadError(String),

    #[error("Plugin manifest error: {0}")]
    ManifestError(String),

    #[error("Plugin I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

// =============================================================================
// Plugin Manager
// =============================================================================

/// Entry for a loaded plugin with its configuration and status.
#[allow(dead_code)]
struct PluginEntry {
    plugin: Box<dyn Plugin>,
    config: PluginConfig,
    effective_priority: PluginPriority,
    load_order: usize,
}

/// Manages the lifecycle of plugins for the cycle engine.
pub struct PluginManager {
    /// Loaded plugins, keyed by name
    plugins: HashMap<String, PluginEntry>,
    /// Order counter for maintaining load order
    load_counter: usize,
    /// Shared context for all plugins (RefCell for interior mutability)
    context: RefCell<PluginContext>,
    /// Whether the manager is initialized
    initialized: bool,
}

impl PluginManager {
    /// Create a new plugin manager.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            load_counter: 0,
            context: RefCell::new(PluginContext::new(
                CycleState {
                    cycle_number: 0,
                    current_phase: CyclePhase::Shadow,
                    phase_started: chrono::Utc::now(),
                    cycle_started: chrono::Utc::now(),
                    phase_day: 0,
                },
                PhaseMetrics {
                    active_agents: 0,
                    spectral_k: 0.0,
                    mean_metabolic_trust: 0.0,
                    active_wounds: 0,
                    composting_entities: 0,
                    liminal_entities: 0,
                    entangled_pairs: 0,
                    held_uncertainties: 0,
                },
            )),
            initialized: false,
        }
    }

    /// Load a plugin into the manager.
    pub fn load_plugin(
        &mut self,
        mut plugin: Box<dyn Plugin>,
        config: PluginConfig,
    ) -> Result<(), PluginError> {
        let name = plugin.name().to_string();

        if self.plugins.contains_key(&name) {
            return Err(PluginError::AlreadyLoaded(name));
        }

        if !config.enabled {
            info!(plugin = %name, "Plugin disabled, not loading");
            return Ok(());
        }

        // Determine effective priority
        let effective_priority = config.priority.unwrap_or_else(|| plugin.priority());

        // Configure the plugin
        plugin.configure(config.clone())?;

        // Call on_load hook
        plugin.on_load(&mut *self.context.borrow_mut())?;

        let load_order = self.load_counter;
        self.load_counter += 1;

        info!(
            plugin = %name,
            version = %plugin.version(),
            priority = ?effective_priority,
            "Plugin loaded"
        );

        self.plugins.insert(
            name,
            PluginEntry {
                plugin,
                config,
                effective_priority,
                load_order,
            },
        );

        Ok(())
    }

    /// Unload a plugin from the manager.
    pub fn unload_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        let mut entry = self
            .plugins
            .remove(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        entry.plugin.on_unload(&mut *self.context.borrow_mut())?;

        info!(plugin = %name, "Plugin unloaded");

        Ok(())
    }

    /// Get a reference to a plugin by name.
    pub fn get_plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(|e| e.plugin.as_ref())
    }

    /// Get a mutable reference to a plugin by name.
    pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut (dyn Plugin + 'static)> {
        self.plugins.get_mut(name).map(|e| e.plugin.as_mut())
    }

    /// Get a typed reference to a plugin.
    pub fn get_plugin_as<T: 'static>(&self, name: &str) -> Option<&T> {
        self.plugins
            .get(name)
            .and_then(|e| e.plugin.as_any().downcast_ref::<T>())
    }

    /// Check if a plugin is loaded.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// Get the names of all loaded plugins.
    pub fn loaded_plugins(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Get the status of all plugins.
    pub fn plugin_statuses(&self) -> HashMap<String, PluginStatus> {
        self.plugins
            .iter()
            .map(|(name, entry)| (name.clone(), entry.plugin.status()))
            .collect()
    }

    /// Update the context with new engine state.
    pub fn update_context(&mut self, state: CycleState, metrics: PhaseMetrics) {
        self.context.borrow_mut().update_cycle_state(state);
        self.context.borrow_mut().update_phase_metrics(metrics);
    }

    /// Get plugins sorted by priority for hook execution.
    fn sorted_plugins(&self) -> Vec<String> {
        let mut entries: Vec<_> = self.plugins.iter().collect();
        entries.sort_by(|a, b| {
            a.1.effective_priority
                .cmp(&b.1.effective_priority)
                .then_with(|| a.1.load_order.cmp(&b.1.load_order))
        });
        entries.into_iter().map(|(name, _)| name.clone()).collect()
    }

    // =========================================================================
    // Hook dispatch methods
    // =========================================================================

    /// Dispatch on_phase_enter to all plugins.
    pub fn dispatch_phase_enter(&mut self, phase: CyclePhase) -> Vec<LivingProtocolEvent> {
        let plugin_order = self.sorted_plugins();
        let mut all_events = Vec::new();

        for name in plugin_order {
            if let Some(entry) = self.plugins.get(&name) {
                entry.plugin.on_phase_enter(phase, &mut *self.context.borrow_mut());
            }
            // Collect emitted events and logs after releasing the plugins borrow
            all_events.extend(self.context.borrow_mut().take_emitted_events());
            for msg in self.context.borrow_mut().take_log_messages() {
                self.dispatch_log_message(&name, msg);
            }
        }

        all_events
    }

    /// Dispatch on_phase_exit to all plugins.
    pub fn dispatch_phase_exit(&mut self, phase: CyclePhase) -> Vec<LivingProtocolEvent> {
        let plugin_order = self.sorted_plugins();
        let mut all_events = Vec::new();

        for name in plugin_order {
            if let Some(entry) = self.plugins.get(&name) {
                entry.plugin.on_phase_exit(phase, &mut *self.context.borrow_mut());
            }
            all_events.extend(self.context.borrow_mut().take_emitted_events());
            for msg in self.context.borrow_mut().take_log_messages() {
                self.dispatch_log_message(&name, msg);
            }
        }

        all_events
    }

    /// Dispatch on_tick to all plugins.
    pub fn dispatch_tick(&mut self) -> Vec<LivingProtocolEvent> {
        let plugin_order = self.sorted_plugins();
        let mut all_events = Vec::new();

        for name in plugin_order {
            if let Some(entry) = self.plugins.get(&name) {
                entry.plugin.on_tick(&mut *self.context.borrow_mut());
            }
            all_events.extend(self.context.borrow_mut().take_emitted_events());
            for msg in self.context.borrow_mut().take_log_messages() {
                self.dispatch_log_message(&name, msg);
            }
        }

        all_events
    }

    /// Dispatch on_event to all plugins.
    pub fn dispatch_event(&mut self, event: &LivingProtocolEvent) {
        let plugin_order = self.sorted_plugins();

        for name in plugin_order {
            if let Some(entry) = self.plugins.get(&name) {
                entry.plugin.on_event(event, &mut *self.context.borrow_mut());
            }
            for msg in self.context.borrow_mut().take_log_messages() {
                self.dispatch_log_message(&name, msg);
            }
        }
    }

    /// Dispatch on_phase_transition to all plugins.
    pub fn dispatch_phase_transition(
        &mut self,
        from: CyclePhase,
        to: CyclePhase,
    ) -> Vec<LivingProtocolEvent> {
        let plugin_order = self.sorted_plugins();
        let mut all_events = Vec::new();

        for name in plugin_order {
            if let Some(entry) = self.plugins.get(&name) {
                entry.plugin.on_phase_transition(from, to, &mut *self.context.borrow_mut());
            }
            all_events.extend(self.context.borrow_mut().take_emitted_events());
            for msg in self.context.borrow_mut().take_log_messages() {
                self.dispatch_log_message(&name, msg);
            }
        }

        all_events
    }

    /// Dispatch on_cycle_start to all plugins.
    pub fn dispatch_cycle_start(&mut self, cycle_number: u64) -> Vec<LivingProtocolEvent> {
        let plugin_order = self.sorted_plugins();
        let mut all_events = Vec::new();

        for name in plugin_order {
            if let Some(entry) = self.plugins.get(&name) {
                entry.plugin.on_cycle_start(cycle_number, &mut *self.context.borrow_mut());
            }
            all_events.extend(self.context.borrow_mut().take_emitted_events());
            for msg in self.context.borrow_mut().take_log_messages() {
                self.dispatch_log_message(&name, msg);
            }
        }

        all_events
    }

    /// Dispatch on_cycle_complete to all plugins.
    pub fn dispatch_cycle_complete(&mut self, cycle_number: u64) -> Vec<LivingProtocolEvent> {
        let plugin_order = self.sorted_plugins();
        let mut all_events = Vec::new();

        for name in plugin_order {
            if let Some(entry) = self.plugins.get(&name) {
                entry.plugin.on_cycle_complete(cycle_number, &mut *self.context.borrow_mut());
            }
            all_events.extend(self.context.borrow_mut().take_emitted_events());
            for msg in self.context.borrow_mut().take_log_messages() {
                self.dispatch_log_message(&name, msg);
            }
        }

        all_events
    }

    /// Check if any plugin requested a pause.
    pub fn is_pause_requested(&self) -> bool {
        self.context.borrow().is_pause_requested()
    }

    /// Process a log message from a plugin.
    fn dispatch_log_message(&self, plugin_name: &str, msg: PluginLogMessage) {
        match msg.level {
            LogLevel::Trace => debug!(plugin = %plugin_name, "{}", msg.message),
            LogLevel::Debug => debug!(plugin = %plugin_name, "{}", msg.message),
            LogLevel::Info => info!(plugin = %plugin_name, "{}", msg.message),
            LogLevel::Warn => warn!(plugin = %plugin_name, "{}", msg.message),
            LogLevel::Error => error!(plugin = %plugin_name, "{}", msg.message),
        }
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for PluginManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginManager")
            .field("plugins", &self.loaded_plugins())
            .field("initialized", &self.initialized)
            .finish()
    }
}

// =============================================================================
// Dynamic Plugin Loading (Feature-Gated)
// =============================================================================

/// Plugin manifest format (plugin.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata
    pub plugin: PluginMetadata,
    /// Dependencies on other plugins
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    /// Default configuration
    #[serde(default)]
    pub config: PluginConfig,
}

/// Metadata from plugin.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name (must match Plugin::name())
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Author(s)
    #[serde(default)]
    pub authors: Vec<String>,
    /// License
    pub license: Option<String>,
    /// Minimum engine version required
    pub min_engine_version: Option<String>,
    /// Library filename (without platform-specific extension)
    pub library: String,
    /// Entry point function name (default: "create_plugin")
    #[serde(default = "default_entry_point")]
    pub entry_point: String,
}

fn default_entry_point() -> String {
    "create_plugin".to_string()
}

/// A dependency on another plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Plugin name
    pub name: String,
    /// Version requirement (semver)
    pub version: String,
    /// Whether this dependency is optional
    #[serde(default)]
    pub optional: bool,
}

impl PluginManifest {
    /// Load a manifest from a TOML file.
    pub fn from_file(path: &std::path::Path) -> Result<Self, PluginError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| PluginError::ManifestError(e.to_string()))
    }

    /// Load a manifest from a TOML string.
    pub fn from_str(content: &str) -> Result<Self, PluginError> {
        toml::from_str(content).map_err(|e| PluginError::ManifestError(e.to_string()))
    }
}

/// Dynamic plugin loader for loading plugins from shared libraries.
///
/// Only available with the `dynamic-plugins` feature.
#[cfg(feature = "dynamic-plugins")]
pub mod dynamic {
    use super::*;
    use libloading::{Library, Symbol};
    use std::path::Path;

    /// Type signature for the plugin entry point function.
    pub type CreatePluginFn = unsafe extern "C" fn() -> *mut dyn Plugin;

    /// A dynamically loaded plugin.
    pub struct DynamicPlugin {
        /// The loaded library (kept alive to prevent unloading)
        _library: Library,
        /// The plugin instance
        plugin: Box<dyn Plugin>,
    }

    impl DynamicPlugin {
        /// Load a plugin from a shared library.
        ///
        /// # Safety
        ///
        /// This function loads and executes code from an external library.
        /// The library must:
        /// - Be compiled with the same Rust version and ABI
        /// - Export a `create_plugin` function with the correct signature
        /// - Not contain malicious code
        pub unsafe fn load(
            library_path: &Path,
            entry_point: &str,
        ) -> Result<Self, PluginError> {
            let library = Library::new(library_path)
                .map_err(|e| PluginError::DynamicLoadError(e.to_string()))?;

            let create_fn: Symbol<CreatePluginFn> = library
                .get(entry_point.as_bytes())
                .map_err(|e| PluginError::DynamicLoadError(e.to_string()))?;

            let plugin_ptr = create_fn();
            let plugin = Box::from_raw(plugin_ptr);

            Ok(Self {
                _library: library,
                plugin,
            })
        }

        /// Get the plugin instance.
        pub fn plugin(&self) -> &dyn Plugin {
            self.plugin.as_ref()
        }

        /// Get a mutable reference to the plugin instance.
        pub fn plugin_mut(&mut self) -> &mut dyn Plugin {
            self.plugin.as_mut()
        }

        /// Take ownership of the plugin (consumes the DynamicPlugin).
        pub fn into_plugin(self) -> Box<dyn Plugin> {
            self.plugin
        }
    }

    /// Dynamic plugin loader with manifest support.
    pub struct DynamicPluginLoader {
        /// Search paths for plugins
        search_paths: Vec<std::path::PathBuf>,
    }

    impl DynamicPluginLoader {
        /// Create a new loader with the given search paths.
        pub fn new(search_paths: Vec<std::path::PathBuf>) -> Self {
            Self { search_paths }
        }

        /// Add a search path.
        pub fn add_search_path(&mut self, path: impl Into<std::path::PathBuf>) {
            self.search_paths.push(path.into());
        }

        /// Load a plugin from a manifest file.
        ///
        /// # Safety
        ///
        /// See `DynamicPlugin::load` for safety requirements.
        pub unsafe fn load_from_manifest(
            &self,
            manifest_path: &Path,
        ) -> Result<(PluginManifest, DynamicPlugin), PluginError> {
            let manifest = PluginManifest::from_file(manifest_path)?;

            // Determine library path
            let library_name = Self::platform_library_name(&manifest.plugin.library);
            let manifest_dir = manifest_path.parent().unwrap_or(Path::new("."));
            let library_path = manifest_dir.join(&library_name);

            if !library_path.exists() {
                return Err(PluginError::DynamicLoadError(format!(
                    "Library not found: {}",
                    library_path.display()
                )));
            }

            let plugin = DynamicPlugin::load(&library_path, &manifest.plugin.entry_point)?;

            // Verify plugin name matches manifest
            if plugin.plugin().name() != manifest.plugin.name {
                return Err(PluginError::ManifestError(format!(
                    "Plugin name mismatch: manifest says '{}', plugin reports '{}'",
                    manifest.plugin.name,
                    plugin.plugin().name()
                )));
            }

            Ok((manifest, plugin))
        }

        /// Get the platform-specific library filename.
        fn platform_library_name(base_name: &str) -> String {
            #[cfg(target_os = "windows")]
            {
                format!("{}.dll", base_name)
            }
            #[cfg(target_os = "macos")]
            {
                format!("lib{}.dylib", base_name)
            }
            #[cfg(target_os = "linux")]
            {
                format!("lib{}.so", base_name)
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            {
                format!("lib{}.so", base_name)
            }
        }

        /// Discover plugins in the search paths.
        pub fn discover_plugins(&self) -> Vec<std::path::PathBuf> {
            let mut manifests = Vec::new();

            for search_path in &self.search_paths {
                if let Ok(entries) = std::fs::read_dir(search_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let manifest_path = path.join("plugin.toml");
                            if manifest_path.exists() {
                                manifests.push(manifest_path);
                            }
                        }
                    }
                }
            }

            manifests
        }
    }

    impl Default for DynamicPluginLoader {
        fn default() -> Self {
            Self::new(Vec::new())
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// Test plugin that tracks hook calls.
    struct TestPlugin {
        name: String,
        call_count: Arc<AtomicU32>,
    }

    impl TestPlugin {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                call_count: Arc::new(AtomicU32::new(0)),
            }
        }

        fn call_count(&self) -> u32 {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn on_tick(&self, _ctx: &mut PluginContext) {
            self.call_count.fetch_add(1, Ordering::SeqCst);
        }

        fn on_phase_enter(&self, _phase: CyclePhase, _ctx: &mut PluginContext) {
            self.call_count.fetch_add(1, Ordering::SeqCst);
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_plugin_manager_load_unload() {
        let mut manager = PluginManager::new();

        let plugin = Box::new(TestPlugin::new("test-plugin"));
        manager
            .load_plugin(plugin, PluginConfig::enabled())
            .unwrap();

        assert!(manager.is_loaded("test-plugin"));
        assert_eq!(manager.loaded_plugins().len(), 1);

        manager.unload_plugin("test-plugin").unwrap();
        assert!(!manager.is_loaded("test-plugin"));
    }

    #[test]
    fn test_plugin_manager_dispatch_tick() {
        let mut manager = PluginManager::new();

        let plugin = TestPlugin::new("tick-test");
        let call_count = plugin.call_count.clone();

        manager
            .load_plugin(Box::new(plugin), PluginConfig::enabled())
            .unwrap();

        manager.dispatch_tick();
        manager.dispatch_tick();
        manager.dispatch_tick();

        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_plugin_manager_dispatch_phase_enter() {
        let mut manager = PluginManager::new();

        let plugin = TestPlugin::new("phase-test");
        let call_count = plugin.call_count.clone();

        manager
            .load_plugin(Box::new(plugin), PluginConfig::enabled())
            .unwrap();

        manager.dispatch_phase_enter(CyclePhase::Shadow);

        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_plugin_priority_ordering() {
        let mut manager = PluginManager::new();

        // Load plugins with different priorities
        let low_priority = Box::new(TestPlugin::new("low"));
        let high_priority = Box::new(TestPlugin::new("high"));
        let normal_priority = Box::new(TestPlugin::new("normal"));

        manager
            .load_plugin(
                low_priority,
                PluginConfig::enabled().with_priority(PluginPriority::Low),
            )
            .unwrap();
        manager
            .load_plugin(
                high_priority,
                PluginConfig::enabled().with_priority(PluginPriority::High),
            )
            .unwrap();
        manager
            .load_plugin(normal_priority, PluginConfig::enabled())
            .unwrap();

        // Check sorted order
        let sorted = manager.sorted_plugins();
        assert_eq!(sorted[0], "high");
        assert_eq!(sorted[1], "normal");
        assert_eq!(sorted[2], "low");
    }

    #[test]
    fn test_plugin_config() {
        let config = PluginConfig::enabled()
            .with_settings(serde_json::json!({
                "threshold": 0.5,
                "enabled": true,
                "name": "test"
            }))
            .with_priority(PluginPriority::High);

        assert!(config.enabled);
        assert_eq!(config.priority, Some(PluginPriority::High));
        assert_eq!(config.get_setting::<f64>("threshold"), Some(0.5));
        assert_eq!(config.get_setting::<bool>("enabled"), Some(true));
        assert_eq!(config.get_setting::<String>("name"), Some("test".to_string()));
    }

    #[test]
    fn test_plugin_context_metadata() {
        let mut ctx = PluginContext::new(
            CycleState {
                cycle_number: 1,
                current_phase: CyclePhase::Shadow,
                phase_started: chrono::Utc::now(),
                cycle_started: chrono::Utc::now(),
                phase_day: 0,
            },
            PhaseMetrics {
                active_agents: 10,
                spectral_k: 0.5,
                mean_metabolic_trust: 0.7,
                active_wounds: 2,
                composting_entities: 1,
                liminal_entities: 3,
                entangled_pairs: 5,
                held_uncertainties: 2,
            },
        );

        ctx.set_metadata("my-key", serde_json::json!({"value": 42}));
        assert!(ctx.get_metadata("my-key").is_some());
        assert_eq!(
            ctx.get_metadata("my-key").unwrap()["value"],
            serde_json::json!(42)
        );

        ctx.remove_metadata("my-key");
        assert!(ctx.get_metadata("my-key").is_none());
    }

    #[test]
    fn test_disabled_plugin_not_loaded() {
        let mut manager = PluginManager::new();

        let plugin = Box::new(TestPlugin::new("disabled-plugin"));
        manager
            .load_plugin(plugin, PluginConfig::disabled())
            .unwrap();

        assert!(!manager.is_loaded("disabled-plugin"));
    }

    #[test]
    fn test_plugin_manifest_parsing() {
        let toml = r#"
[plugin]
name = "example-plugin"
version = "1.0.0"
description = "An example plugin"
authors = ["Test Author"]
license = "MIT"
library = "example_plugin"

[[dependencies]]
name = "other-plugin"
version = ">=1.0.0"

[config]
enabled = true

[config.settings]
threshold = 0.5
"#;

        let manifest = PluginManifest::from_str(toml).unwrap();
        assert_eq!(manifest.plugin.name, "example-plugin");
        assert_eq!(manifest.plugin.version, "1.0.0");
        assert_eq!(manifest.plugin.library, "example_plugin");
        assert_eq!(manifest.dependencies.len(), 1);
        assert!(manifest.config.enabled);
    }
}
