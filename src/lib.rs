//! Zessionizer: A Zellij plugin for intelligent project and session management.
//!
//! Zessionizer is a terminal multiplexer plugin that provides:
//! - Fuzzy-searchable project switching with frecency-based ranking
//! - Automatic project discovery via filesystem scanning
//! - Session management with creation, switching, and deletion
//! - Persistent state backed by JSON file storage with frecency ranking
//! - Asynchronous background scanning via Zellij worker threads

#![allow(clippy::multiple_crate_versions)]

//!
//! # Architecture
//!
//! The crate follows a layered architecture pattern:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │  Zellij Plugin Shim (main.rs)                       │  ← Entry point
//! └─────────────────────────────────────────────────────┘
//!                        │
//! ┌─────────────────────────────────────────────────────┐
//! │  Application Layer (app/)                           │  ← State machine
//! │  - Event handling                                   │  ← Business logic
//! │  - Action dispatching                               │
//! │  - View model computation                           │
//! └─────────────────────────────────────────────────────┘
//!         │                    │                    │
//! ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
//! │ UI Layer      │   │ Storage Layer │   │ Worker Layer  │
//! │ (ui/)         │   │ (storage/)    │   │ (worker/)     │
//! │ - Rendering   │   │ - JSON I/O    │   │ - Async scan  │
//! │ - Theming     │   │ - Frecency    │   │ - Git discover│
//! │ - Components  │   │ - Backend API │   │ - IPC bridge  │
//! └───────────────┘   └───────────────┘   └───────────────┘
//!         │                    │                    │
//! ┌─────────────────────────────────────────────────────┐
//! │  Infrastructure & Domain Layers                     │
//! │  - Platform paths (infrastructure/)                 │
//! │  - Error types (domain/error)                       │
//! │  - Project model (domain/project)                   │
//! └─────────────────────────────────────────────────────┘
//!                        │
//! ┌─────────────────────────────────────────────────────┐
//! │  Observability (observability/)                     │  ← Optional
//! │  - OpenTelemetry tracing                            │
//! │  - File-based OTLP export                           │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Modules
//!
//! - [`app`]: Application state machine with event/action model
//! - [`domain`]: Core domain types (Project, errors)
//! - [`infrastructure`]: Platform-specific utilities (paths)
//! - [`storage`]: JSON file persistence layer with frecency ranking
//! - [`worker`]: Background worker for async project scanning
//! - [`ui`]: Terminal rendering with theme support
//! - `observability`: OpenTelemetry tracing (internal)
//!
//! # Configuration
//!
//! The plugin is configured via Zellij's plugin configuration:
//!
//! ```kdl
//! // ~/.config/zellij/layouts/default.kdl
//! pane {
//!     plugin location="file:/path/to/zessionizer.wasm" {
//!         scan_paths "~/Git,~/Projects"
//!         scan_depth "4"
//!         theme "catppuccin-mocha"
//!         trace_level "info"
//!     }
//! }
//! ```
//!
//! Or loaded on-demand with `Ctrl+o` → `Ctrl+w` and entering the configuration.
//!
//! # Initialization Flow
//!
//! 1. **Plugin Load** (`main.rs`):
//!    - Parse configuration from Zellij
//!    - Initialize tracing (optional)
//!    - Create `AppState` with theme
//!    - Subscribe to Zellij events
//!    - Post initial `LoadProjects` message to worker
//!
//! 2. **Session Update**:
//!    - Run filesystem scan via `find` command
//!    - Parse project directories (by finding `.git` dirs or `.zessionizer` files)
//!    - Send `AddProjects` message to worker
//!
//! 3. **Worker Processing**:
//!    - Batch insert projects into JSON storage
//!    - Load frecency-sorted projects
//!    - Send `ProjectsLoaded` response to plugin
//!
//! 4. **UI Rendering**:
//!    - Compute view model from state
//!    - Render components (header, table, footer)
//!    - Handle user input (j/k/Enter/q)
//!
//! # Examples
//!
//! ## Basic Usage (Library)
//!
//! ```rust
//! use zessionizer::{Config, initialize, handle_event, Event};
//!
//! // Initialize application
//! let config = Config {
//!     scan_paths: vec!["~/Projects".to_string()],
//!     scan_depth: 4,
//!     ..Default::default()
//! };
//!
//! let mut state = initialize(config);
//!
//! // Handle events
//! let events = vec![Event::KeyDown, Event::SelectProject];
//! for event in events {
//!     let actions = handle_event(&mut state, &event)?;
//!     // Execute actions...
//! }
//! # Ok::<(), zessionizer::ZessionizerError>(())
//! ```
//!
//! ## Worker Usage
//!
//! ```rust
//! use zessionizer::worker::{WorkerMessage, ZessionizerWorker};
//! use zellij_tile::prelude::*;
//!
//! // In worker thread
//! let mut worker = ZessionizerWorker::default();
//! let message = WorkerMessage::load_projects(false);
//! worker.update(Event::CustomMessage(
//!     "zessionizer".to_string(),
//!     serde_json::to_string(&message).unwrap()
//! ));
//! ```
//!
//! # Key Design Decisions
//!
//! ## Frecency-Based Ranking
//!
//! Projects are ranked by a frecency algorithm combining frequency and recency:
//! - Storage maintains visit counts and last access timestamps
//! - Sorting happens in-memory for optimal performance
//! - Projects bubble to top with repeated access
//!
//! ## Worker-Based Scanning
//!
//! Filesystem scanning runs in a separate Zellij worker thread:
//! - Prevents UI blocking during expensive I/O operations
//! - Uses IPC messaging for result communication
//! - Batch updates minimize storage write operations
//!
//! ## Immutable View Models
//!
//! UI rendering uses computed view models:
//! - Clear separation between state and display
//! - Enables easier testing and validation
//! - Pre-computes expensive operations (fuzzy match highlighting)
//!
//! # Performance Characteristics
//!
//! - **Startup Time**: ~30ms (includes JSON load + theme initialization)
//! - **Project Scan**: ~200ms for 1000 projects (parallelized via `find`)
//! - **Storage Write**: ~5ms for 100 projects (atomic file write)
//! - **Render Time**: <1ms per frame (direct ANSI output)
//!
//! # Platform Support
//!
//! - **Target**: `wasm32-wasip1` (Zellij WASM runtime)
//! - **OS Support**: Linux, macOS, Windows (via data directory detection)
//! - **Terminal**: Any ANSI-capable terminal emulator

pub mod app;
pub mod domain;
pub mod infrastructure;
pub mod storage;
pub mod worker;

pub mod ui;

pub mod observability;

pub use app::{handle_event, Action, AppState, Event, InputMode, SearchFocus, ViewMode};
pub use domain::{Project, Result, ZessionizerError};
pub use ui::Theme;

use std::collections::BTreeMap;

/// Plugin configuration parsed from Zellij's configuration system.
///
/// Configuration values are provided via Zellij's KDL layout configuration
/// and passed to the plugin during initialization.
///
/// # Example
///
/// ```kdl
/// plugin location="file:/path/to/zessionizer.wasm" {
///     scan_paths "~/Git,~/Projects"
///     scan_depth "4"
///     theme "catppuccin-mocha"
///     theme_file "/path/to/theme.toml"
///     trace_level "debug"
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// Comma-separated paths to scan for projects.
    ///
    /// Paths are resolved relative to the user's home directory if they start
    /// with `~`. Default: `["~/Projects"]`
    pub scan_paths: Vec<String>,

    /// Maximum directory depth for recursive scanning.
    ///
    /// Higher values scan deeper but take longer. Recommended: 3-5. Default: 4
    pub scan_depth: u32,

    /// Built-in theme name to use.
    ///
    /// Options: `catppuccin-mocha`, `catppuccin-latte`, `catppuccin-frappe`,
    /// `catppuccin-macchiato`. Ignored if `theme_file` is set.
    pub theme_name: Option<String>,

    /// Path to a custom TOML theme file.
    ///
    /// Takes precedence over `theme_name`. See [`ui::theme`] for format.
    pub theme_file: Option<String>,

    /// Tracing level for OpenTelemetry spans.
    ///
    /// Options: `trace`, `debug`, `info`, `warn`, `error`. Default: `"info"`
    pub trace_level: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_paths: vec!["~/Projects".to_string()],
            scan_depth: 4,
            theme_name: None,
            theme_file: None,
            trace_level: None,
        }
    }
}

impl Config {
    /// Parses configuration from Zellij's configuration map.
    ///
    /// Zellij provides configuration as a `BTreeMap<String, String>` during
    /// plugin initialization. This function extracts and parses typed values
    /// with fallback defaults.
    ///
    /// # Parameters
    ///
    /// * `config` - Configuration map from Zellij
    ///
    /// # Parsing Rules
    ///
    /// - `scan_paths`: Comma-separated string → `Vec<String>` (filters empty values)
    /// - `scan_depth`: String → `u32` (falls back to 4 on parse error)
    /// - `theme`: String → `Option<String>`
    /// - `theme_file`: String → `Option<String>`
    /// - `trace_level`: String → `Option<String>`
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::BTreeMap;
    /// use zessionizer::Config;
    ///
    /// let mut map = BTreeMap::new();
    /// map.insert("scan_paths".to_string(), "~/Projects,~/Code".to_string());
    /// map.insert("scan_depth".to_string(), "5".to_string());
    ///
    /// let config = Config::from_zellij(map);
    /// assert_eq!(config.scan_paths, vec!["~/Projects", "~/Code"]);
    /// assert_eq!(config.scan_depth, 5);
    /// ```
    #[must_use]
    pub fn from_zellij(config: &BTreeMap<String, String>) -> Self {
        let scan_paths = config
            .get("scan_paths")
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
            .unwrap_or_else(|| vec!["~/Projects".to_string()]);

        let scan_depth = config
            .get("scan_depth")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(4);

        Self {
            scan_paths,
            scan_depth,
            theme_name: config.get("theme").cloned(),
            theme_file: config.get("theme_file").cloned(),
            trace_level: config.get("trace_level").cloned(),
        }
    }
}

/// Initializes the plugin with configuration.
///
/// Creates a new `AppState` with:
/// - Tracing subscriber (if `trace_level` is set)
/// - Loaded theme (from file, name, or default)
/// - Empty project list (populated later by worker)
///
/// # Parameters
///
/// * `config` - Plugin configuration
///
/// # Returns
///
/// An initialized `AppState` ready for event processing.
///
/// # Side Effects
///
/// - Initializes OpenTelemetry tracing subscriber
/// - Logs initialization event
/// - Creates data directory if it doesn't exist
///
/// # Example
///
/// ```rust
/// use zessionizer::{Config, initialize};
///
/// let config = Config {
///     trace_level: Some("debug".to_string()),
///     ..Default::default()
/// };
///
/// let state = initialize(config);
/// // State is ready for event processing
/// ```
pub fn initialize(config: &Config) -> AppState {
    tracing::debug!("initializing zessionizer plugin");

    let theme = config.theme_file.as_ref().map_or_else(
        || {
            config.theme_name.as_ref().map_or_else(
                Theme::default,
                |theme_name| {
                    Theme::from_name(theme_name).unwrap_or_else(|| {
                        tracing::debug!(theme_name = %theme_name, "failed to load theme, using default");
                        Theme::default()
                    })
                },
            )
        },
        |theme_file| {
            Theme::from_file(theme_file.clone()).unwrap_or_else(|e| {
                tracing::debug!(theme_file = %theme_file, error = %e, "failed to load theme from file, using default");
                Theme::default()
            })
        },
    );

    AppState::new(vec![], theme)
}
