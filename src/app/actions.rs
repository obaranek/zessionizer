//! Actions representing side effects to be executed by the plugin runtime.
//!
//! This module defines the [`Action`] type, which represents imperative commands
//! produced by the event handler after processing user input or system events.
//! Actions bridge pure state transformations and effectful operations like
//! rendering UI, managing sessions, or communicating with background workers.
//!
//! # Architecture
//!
//! The event handler returns a `Vec<Action>` after processing each event, allowing
//! multiple side effects to be queued atomically. The plugin runtime executes
//! these actions in sequence via the action processor.
//!
//! # Example
//!
//! ```rust
//! use crate::app::Action;
//! use crate::worker::WorkerMessage;
//! use std::path::PathBuf;
//!
//! let actions = vec![
//!     Action::PostToWorker(WorkerMessage::load_projects(false)),
//! ];
//! ```

use crate::worker::WorkerMessage;
use std::path::PathBuf;

/// Commands representing side effects to be executed by the plugin runtime.
///
/// Actions are produced by the event handler and executed by the action processor.
/// They represent the boundary between pure state transformations and effectful
/// operations like session management and worker communication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Closes the focused floating pane, hiding the plugin UI.
    ///
    /// Sent when the user explicitly requests to exit the plugin (e.g., pressing 'q').
    CloseFocus,

    /// Posts a message to the background worker thread.
    ///
    /// Enables asynchronous operations like storage queries, frecency updates,
    /// or project batch additions without blocking the main event loop.
    PostToWorker(WorkerMessage),

    /// Switches to an existing Zellij session.
    ///
    /// Changes focus to the specified session without creating a new one. Used
    /// when the user selects a project that already has an active session.
    SwitchSession {
        /// Name of the existing session.
        name: String,
        /// Filesystem path to set as working directory.
        path: PathBuf,
        /// Optional layout to use when switching session.
        layout: Option<String>,
    },

    /// Creates a new Zellij session.
    ///
    /// Spawns a new session with the specified name and working directory. Used
    /// when the user selects a project without an active session.
    CreateSession {
        /// Name for the new session.
        name: String,
        /// Filesystem path to set as working directory.
        path: PathBuf,
        /// Optional layout to use when creating session.
        layout: Option<String>,
    },

    /// Kills an existing Zellij session.
    ///
    /// Terminates the specified session and all its panes. Used when the user
    /// explicitly requests session termination (e.g., pressing 'K' in Sessions view).
    KillSession {
        /// Name of the session to terminate.
        name: String,
    },

    /// Updates the layout associated with a project.
    UpdateProjectLayout {
        /// Path of the project to update.
        path: String,
        /// New layout to associate with the project (None to clear).
        layout: Option<String>,
    },
}
