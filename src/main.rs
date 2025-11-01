//! Zellij plugin wrapper and entry point.
//!
//! This module provides the thin integration layer between the Zessionizer library
//! and the Zellij plugin system. It implements the `ZellijPlugin` and
//! `ZellijWorker` traits to handle Zellij events and lifecycle.
//!
//! # Architecture
//!
//! The plugin uses Zellij's worker thread support for background processing:
//!
//! ```text
//! ┌─────────────────────────┐
//! │   Zellij Main Thread    │
//! │  ┌──────────────────┐   │
//! │  │  State (plugin)  │   │  ← UI state, event handling
//! │  └──────────────────┘   │
//! │          │              │
//! │          │ IPC          │
//! │          ▼              │
//! │  ┌──────────────────┐   │
//! │  │ ZessionizerWorker│   │  ← Background processing
//! │  │ (worker thread)  │   │  ← Storage operations
//! │  └──────────────────┘   │
//! └─────────────────────────┘
//! ```
//!
//! # Plugin Lifecycle
//!
//! 1. **Load**: Parse config, initialize tracing, create `AppState`
//! 2. **Subscribe**: Register for Key, `SessionUpdate`, `CustomMessage`, `Timer` events
//! 3. **Initial Scan**: Run `find` command to discover projects
//! 4. **Periodic Scan**: Re-scan filesystem on timer intervals
//! 5. **Update**: Handle events, delegate to library layer
//! 6. **Render**: Call library render function
//!
//! # Worker Communication
//!
//! Messages between plugin and worker use JSON serialization:
//!
//! - Plugin → Worker: [`WorkerMessage`] (`LoadProjects`, `AddProjects`, etc.)
//! - Worker → Plugin: [`WorkerResponse`] (`ProjectsLoaded`, error details)
//!
//! # Event Mapping
//!
//! Zellij events are translated to library events:
//!
//! - `Key(Down)` → `Event::KeyDown`
//! - `Key(Enter)` → `Event::SelectProject` (unless typing in search)
//! - `Key(Esc)` → `Event::ExitSearch` (in search mode)
//! - `SessionUpdate` → `Event::SessionUpdate { active_sessions }`
//! - `RunCommandResult` → `Event::ProjectsScanned { git_directories }`
//!
//! # Keybindings
//!
//! Global (all modes):
//! - `Ctrl+n`: Move down
//! - `Ctrl+p`: Move up
//!
//! In normal mode:
//! - `j`/`Down`: Move down
//! - `k`/`Up`: Move up
//! - `Enter`: Select project
//! - `q`: Close plugin
//! - `/`: Enter search mode
//! - `n`: Show projects view
//! - `s`: Show sessions view
//! - `K` (shift): Kill selected session
//!
//! In search mode:
//! - `j`/`k`/etc.: Type characters
//! - `Enter`: Select project
//! - `Esc`: Exit search
//! - `/`: Return to search input

#![allow(clippy::multiple_crate_versions)]

use std::collections::BTreeMap;
use zellij_tile::prelude::*;
use zellij_tile::shim::post_message_to;

use zessionizer::worker::{WorkerMessage, WorkerResponse, ZessionizerWorker};
use zessionizer::{handle_event, Action, Config, Event, InputMode};

// Register plugin and worker with Zellij
register_plugin!(State);
register_worker!(ZessionizerWorker, zessionizer_worker, ZESSIONIZER_WORKER);

/// Plugin state wrapper.
///
/// Wraps the library's `AppState` with Zellij-specific concerns like worker
/// communication and scan tracking.
struct State {
    /// Core application state from library layer.
    app: zessionizer::app::AppState,

    /// Worker thread identifier for IPC messaging.
    worker_name: String,

    /// Configured scan paths (for `find` command).
    scan_paths: Vec<String>,

    /// Configured scan depth (for `find` command).
    scan_depth: u32,
}

impl Default for State {
    fn default() -> Self {
        let default_config = Config::default();
        Self {
            app: zessionizer::initialize(&default_config),
            worker_name: "zessionizer".to_string(),
            scan_paths: Vec::new(),
            scan_depth: 4,
        }
    }
}

impl ZellijPlugin for State {
    /// Initializes the plugin on load.
    ///
    /// Called once during plugin startup. Parses configuration, initializes
    /// application state, requests permissions, subscribes to events, and
    /// posts initial worker message.
    ///
    /// # Tracing
    ///
    /// The entire load process is instrumented with OpenTelemetry spans.
    ///
    /// # Permissions
    ///
    /// Requests:
    /// - `ReadApplicationState`: Read session info
    /// - `ChangeApplicationState`: Switch/create/kill sessions
    /// - `RunCommands`: Execute `find` for project scanning
    /// - `FullHdAccess`: Read filesystem for Git directories
    ///
    /// # Subscriptions
    ///
    /// - `Key`: Keyboard input
    /// - `SessionUpdate`: Session lifecycle changes
    /// - `CustomMessage`: Worker responses
    /// - `RunCommandResult`: `find` command output
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        let config = Config::from_zellij(&configuration);
        zessionizer::observability::init_tracing(&config);

        let span = tracing::debug_span!("plugin_load");
        let _guard = span.entered();

        tracing::debug!("plugin loading started");
        tracing::debug!(scan_paths = ?config.scan_paths, "parsed configuration");
        self.app = zessionizer::initialize(&config);
        tracing::debug!("app state initialized");

        tracing::debug!("requesting permissions");
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
            PermissionType::FullHdAccess,
        ]);

        tracing::debug!("subscribing to events");
        subscribe(&[
            EventType::Key,
            EventType::SessionUpdate,
            EventType::CustomMessage,
            EventType::RunCommandResult,
            EventType::PermissionRequestResult,
            EventType::FileSystemCreate,
            EventType::FileSystemUpdate,
            EventType::FileSystemDelete,
        ]);

        self.scan_paths.clone_from(&config.scan_paths);
        self.scan_depth = config.scan_depth;

        tracing::debug!("plugin load complete - waiting for permissions");
    }

    /// Handles incoming Zellij events.
    ///
    /// Translates Zellij events to library events, delegates to `handle_event`,
    /// and executes resulting actions. Returns `true` if the UI should re-render.
    ///
    /// # Filesystem Scanning
    ///
    /// Periodic `Timer` events trigger filesystem scans via `find` command to discover
    /// Git repositories in configured scan paths. Results are sent to the worker
    /// for batch insertion.
    ///
    /// # Tracing
    ///
    /// Each event is traced with its type for observability.
    ///
    /// # Parameters
    ///
    /// * `event` - Zellij event to process
    ///
    /// # Returns
    ///
    /// - `true` if the plugin UI should re-render
    /// - `false` if the event was ignored or resulted in no state changes
    fn update(&mut self, event: zellij_tile::prelude::Event) -> bool {
        let event_name = Self::get_event_name(&event);
        let span_name = format!("plugin_update::{event_name}");
        let span = tracing::debug_span!("plugin_update_event", otel.name = %span_name, event_type = %event_name);
        let _guard = span.entered();

        tracing::debug!(event = %event_name, "processing event");

        let our_event = match event {
            zellij_tile::prelude::Event::Key(ref key) => match self.map_key_event(key) {
                Some(event) => event,
                None => return false,
            },
            zellij_tile::prelude::Event::CustomMessage(message, payload) => {
                match self.map_custom_message_event(&message, &payload) {
                    Some(event) => event,
                    None => return false,
                }
            }
            zellij_tile::prelude::Event::RunCommandResult(exit_code, stdout, stderr, _context) => {
                Self::map_command_result_event(exit_code, stdout, stderr)
            }
            zellij_tile::prelude::Event::SessionUpdate(session_infos, _resurrectable_sessions) => {
                Self::map_session_update_event(&session_infos)
            }
            zellij_tile::prelude::Event::FileSystemCreate(paths)
            | zellij_tile::prelude::Event::FileSystemUpdate(paths)
            | zellij_tile::prelude::Event::FileSystemDelete(paths) => {
                tracing::debug!(
                    path_count = paths.len(),
                    "filesystem change detected - triggering scan"
                );
                self.trigger_filesystem_scan();
                return false;
            }
            zellij_tile::prelude::Event::PermissionRequestResult(permissions) => {
                self.handle_permission_result(permissions);
                return false;
            }
            _ => return false,
        };

        match handle_event(&mut self.app, &our_event) {
            Ok((should_render, actions)) => {
                tracing::debug!(
                    action_count = actions.len(),
                    should_render = should_render,
                    "event handled successfully"
                );
                for a in actions {
                    self.execute_action(&a);
                }
                should_render
            }
            Err(e) => {
                tracing::debug!(error = %e, "error handling event");
                false
            }
        }
    }

    /// Renders the plugin UI.
    ///
    /// Delegates to the library's rendering layer.
    ///
    /// # Parameters
    ///
    /// * `rows` - Terminal height in rows
    /// * `cols` - Terminal width in columns
    fn render(&mut self, rows: usize, cols: usize) {
        zessionizer::ui::render(&self.app, rows, cols);
    }
}

impl State {
    /// Triggers filesystem scan for .git directories and .zessionizer marker files.
    fn trigger_filesystem_scan(&self) {
        tracing::debug!(
            "running find command to scan for .git directories and .zessionizer marker files"
        );

        for scan_path in &self.scan_paths {
            let expanded_path = if scan_path.starts_with("~/") {
                scan_path.strip_prefix("~/").unwrap_or(scan_path)
            } else if scan_path == "~" {
                "."
            } else {
                scan_path.as_str()
            };

            tracing::debug!(scan_path = %scan_path, expanded_path = %expanded_path, "scanning path");

            run_command(
                &[
                    "find",
                    expanded_path,
                    "-maxdepth",
                    &self.scan_depth.to_string(),
                    "(",
                    "-name",
                    ".git",
                    "-type",
                    "d",
                    "-o",
                    "-name",
                    ".zessionizer",
                    "-type",
                    "f",
                    ")",
                ],
                BTreeMap::new(),
            );
        }
    }

    /// Gets a string name for a Zellij event for logging purposes.
    fn get_event_name(event: &zellij_tile::prelude::Event) -> String {
        match event {
            zellij_tile::prelude::Event::Key(key) => format!("Key({:?})", key.bare_key),
            zellij_tile::prelude::Event::CustomMessage(msg, _) => format!("CustomMessage({msg})"),
            zellij_tile::prelude::Event::RunCommandResult(..) => "RunCommandResult".to_string(),
            zellij_tile::prelude::Event::SessionUpdate(..) => "SessionUpdate".to_string(),
            zellij_tile::prelude::Event::PermissionRequestResult(..) => {
                "PermissionRequestResult".to_string()
            }
            zellij_tile::prelude::Event::FileSystemCreate(..) => "FileSystemCreate".to_string(),
            zellij_tile::prelude::Event::FileSystemUpdate(..) => "FileSystemUpdate".to_string(),
            zellij_tile::prelude::Event::FileSystemDelete(..) => "FileSystemDelete".to_string(),
            _ => "Other".to_string(),
        }
    }

    /// Maps keyboard events to application events.
    fn map_key_event(&self, key: &KeyWithModifier) -> Option<Event> {
        tracing::debug!(bare_key = ?key.bare_key, "key event");

        if key.bare_key == BareKey::Char('n') && key.has_modifiers(&[KeyModifier::Ctrl]) {
            return Some(Event::KeyDown);
        }
        if key.bare_key == BareKey::Char('p') && key.has_modifiers(&[KeyModifier::Ctrl]) {
            return Some(Event::KeyUp);
        }

        Some(match key.bare_key {
            BareKey::Down | BareKey::Char('j') => match self.app.input_mode {
                InputMode::Search(_) => Event::Char('j'),
                InputMode::Normal => Event::KeyDown,
            },
            BareKey::Up | BareKey::Char('k') => match self.app.input_mode {
                InputMode::Search(_) => Event::Char('k'),
                InputMode::Normal => Event::KeyUp,
            },
            BareKey::Esc => match self.app.input_mode {
                InputMode::Search(_) => Event::ExitSearch,
                InputMode::Normal => Event::Escape,
            },
            BareKey::Char('q') if self.app.input_mode == InputMode::Normal => Event::CloseFocus,
            BareKey::Char('K') => Event::KillSession,
            BareKey::Enter => Event::SelectProject,
            BareKey::Char('/') => match self.app.input_mode {
                InputMode::Normal => Event::SearchMode,
                InputMode::Search(_) => Event::FocusSearchBar,
            },
            BareKey::Char('n') if self.app.input_mode == InputMode::Normal => Event::ShowProjects,
            BareKey::Char('s') if self.app.input_mode == InputMode::Normal => Event::ShowSessions,
            BareKey::Backspace => Event::Backspace,
            BareKey::Char(c) => Event::Char(c),
            _ => return None,
        })
    }

    /// Handles permission request results.
    fn handle_permission_result(&self, permissions: PermissionStatus) {
        match permissions {
            PermissionStatus::Granted => {
                tracing::debug!("permissions granted - initializing plugin");
                self.post_worker_message(&WorkerMessage::load_projects(false));
                if !self.scan_paths.is_empty() {
                    tracing::debug!("triggering initial filesystem scan");
                    self.trigger_filesystem_scan();
                }
            }
            PermissionStatus::Denied => {
                tracing::warn!("permissions denied - plugin functionality limited");
            }
        }
    }

    /// Maps custom message events to application events.
    fn map_custom_message_event(&self, message: &str, payload: &str) -> Option<Event> {
        tracing::debug!(message_name = %message, payload_len = payload.len(), "custom message event");

        if message == self.worker_name {
            match serde_json::from_str::<WorkerResponse>(payload) {
                Ok(response) => {
                    tracing::debug!(response = ?response, "worker response received");
                    Some(Event::WorkerResponse(response))
                }
                Err(e) => {
                    tracing::debug!(error = %e, "failed to deserialize worker response");
                    None
                }
            }
        } else {
            tracing::debug!(message_name = %message, "ignoring custom message with unknown name");
            None
        }
    }

    /// Maps run command result events to application events.
    fn map_command_result_event(exit_code: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) -> Event {
        tracing::debug!(exit_code = ?exit_code, "run command result event");

        if exit_code == Some(0) {
            let output = String::from_utf8(stdout).unwrap_or_default();
            let git_dirs: Vec<String> = output.lines().map(ToString::to_string).collect();
            tracing::debug!(
                git_directory_count = git_dirs.len(),
                "found git directories"
            );
            Event::ProjectsScanned {
                git_directories: git_dirs,
            }
        } else {
            let error = String::from_utf8(stderr).unwrap_or_default();
            tracing::debug!(error = %error, "find command failed");
            Event::ScanFailed { error }
        }
    }

    /// Maps session update events to application events.
    fn map_session_update_event(session_infos: &[zellij_tile::prelude::SessionInfo]) -> Event {
        tracing::debug!(session_count = session_infos.len(), "session update event");
        let active_sessions = session_infos.iter().map(|s| s.name.clone()).collect();
        let current_session = session_infos
            .iter()
            .find(|s| s.is_current_session)
            .map(|s| s.name.clone());
        Event::SessionUpdate {
            active_sessions,
            current_session,
        }
    }

    /// Posts a message to the worker thread.
    ///
    /// Serializes the message as JSON and sends via Zellij's IPC system.
    ///
    /// # Parameters
    ///
    /// * `message` - Worker message to send
    ///
    /// # Errors
    ///
    /// Logs serialization errors but does not propagate them.
    fn post_worker_message(&self, message: &WorkerMessage) {
        match serde_json::to_string(&message) {
            Ok(payload) => {
                tracing::debug!(payload_len = payload.len(), "posting message to worker");
                post_message_to(PluginMessage {
                    worker_name: Some(self.worker_name.clone()),
                    name: self.worker_name.clone(),
                    payload,
                });
            }
            Err(e) => {
                tracing::debug!(error = %e, "failed to serialize worker message");
            }
        }
    }

    /// Executes an action returned from event handling.
    ///
    /// Translates library actions to Zellij API calls.
    ///
    /// # Actions
    ///
    /// - `CloseFocus`: Close plugin pane
    /// - `SwitchSession`: Switch to existing session and close plugin
    /// - `CreateSession`: Create new session, switch to it, and close plugin
    /// - `KillSession`: Terminate session by name
    /// - `PostToWorker`: Send IPC message to worker thread
    ///
    /// # Parameters
    ///
    /// * `action` - Action to execute
    #[tracing::instrument(level = "debug", skip(self))]
    fn execute_action(&self, action: &Action) {
        match action {
            Action::CloseFocus => {
                tracing::debug!("closing plugin focus");
                hide_self();
            }
            Action::SwitchSession { ref name, ref path } => {
                tracing::debug!(session = %name, path = ?path, "switching to session");

                let path_str = path.to_string_lossy().to_string();
                self.post_worker_message(&WorkerMessage::update_frecency(path_str));
                self.post_worker_message(&WorkerMessage::load_projects(false));

                switch_session_with_cwd(Some(name), Some(path.clone()));
                hide_self();
            }
            Action::CreateSession { ref name, ref path } => {
                tracing::debug!(session = %name, path = ?path, "creating new session");

                let path_str = path.to_string_lossy().to_string();
                self.post_worker_message(&WorkerMessage::update_frecency(path_str));
                self.post_worker_message(&WorkerMessage::load_projects(false));

                switch_session_with_cwd(Some(name), Some(path.clone()));
                hide_self();
            }
            Action::KillSession { ref name } => {
                tracing::debug!(session = %name, "killing session");
                kill_sessions(&[name]);
            }
            Action::PostToWorker(ref message) => {
                tracing::debug!(message = ?message, "posting message to worker");
                self.post_worker_message(message);
            }
        }
    }
}
