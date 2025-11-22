//! Event handling and state transition logic.
//!
//! This module implements the core event handler that processes user input,
//! system events, and worker responses, translating them into state changes
//! and action sequences. It serves as the primary control flow coordinator
//! for the application.
//!
//! # Architecture
//!
//! The handler follows a unidirectional data flow pattern:
//! 1. Events arrive from the plugin runtime or worker thread
//! 2. [`handle_event`] pattern-matches the event type
//! 3. State mutations occur via `AppState` methods
//! 4. Actions are collected and returned for execution
//!
//! # Event Types
//!
//! Events fall into several categories:
//! - **Navigation**: `KeyDown`, `KeyUp`, `SelectProject`
//! - **Input**: `Char`, `Backspace`, `Escape`
//! - **Mode Switching**: `SearchMode`, `ShowProjects`, `ShowSessions`
//! - **System**: `SessionUpdate`, `ProjectsScanned`, `PermissionsResult`
//! - **Worker**: `WorkerResponse` with typed message variants
//!
//! # Example
//!
//! ```rust
//! use crate::app::{AppState, handler::{Event, handle_event}};
//! use crate::ui::theme::Theme;
//!
//! let mut state = AppState::new(vec![], Theme::default());
//! let actions = handle_event(&mut state, &Event::KeyDown)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::app::{Action, AppState};
use crate::domain::error::Result;
use crate::worker::{WorkerMessage, WorkerResponse};
use std::collections::HashSet;
use std::path::PathBuf;
use zellij_tile::prelude::PermissionType;

/// Events triggered by user input, system changes, or worker responses.
///
/// Each event represents a discrete occurrence that may cause state changes
/// and action emissions. The event handler processes these sequentially,
/// ensuring deterministic state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Moves selection cursor down by one position (wraps to top).
    KeyDown,
    /// Moves selection cursor up by one position (wraps to bottom).
    KeyUp,
    /// Closes the floating pane and hides the plugin UI.
    CloseFocus,
    /// Selects the currently highlighted project (creates or switches session).
    SelectProject,
    /// Kills the currently selected session (Sessions view only).
    KillSession,
    /// Enters search mode with typing focus.
    SearchMode,
    /// Focuses the search input field (from navigating mode).
    FocusSearchBar,
    /// Focuses the search results list (from typing mode).
    FocusResults,
    /// Exits search mode and clears the query.
    ExitSearch,
    /// Appends a character to the search query.
    Char(char),
    /// Removes the last character from the search query.
    Backspace,
    /// Clears search query and returns to normal mode.
    Escape,

    /// Switches view to show projects without active sessions.
    ShowProjects,
    /// Switches view to show projects with active sessions.
    ShowSessions,

    /// Updates the set of active Zellij sessions.
    ///
    /// Triggered by periodic polling or session lifecycle events. Causes
    /// project list re-filtering and storage synchronization if changes detected.
    SessionUpdate {
        /// Current set of active session names.
        active_sessions: HashSet<String>,
        /// Name of the current session.
        current_session: Option<String>,
    },

    /// Reports discovered project directories from filesystem scan.
    ///
    /// Triggered after scanning completes. Causes batch project addition
    /// via worker if new directories are found.
    ProjectsScanned {
        /// Paths to marker files (`.git` directories or `.zessionizer` files)
        /// that identify project directories.
        git_directories: Vec<String>,
    },

    /// Reports filesystem scan failure.
    ///
    /// Logged but does not affect application state. User can retry scan
    /// by reopening the plugin.
    ScanFailed {
        /// Error message describing the failure.
        error: String,
    },

    /// Reports granted Zellij permissions after permission request.
    ///
    /// Currently unused but reserved for future permission-dependent features.
    PermissionsResult {
        /// Permissions granted by the user.
        granted: Vec<PermissionType>,
    },

    /// Wraps a response from the background worker thread.
    ///
    /// Processed by matching on the inner [`WorkerResponse`] variant. May
    /// cause project list updates, state changes, or error handling.
    WorkerResponse(WorkerResponse),

    /// Updates the layout associated with a project.
    UpdateProjectLayout {
        /// Path of the project to update.
        path: String,
        /// New layout to associate with the project.
        layout: Option<String>,
    },
}

/// Processes an event, mutates application state, and returns actions to execute.
///
/// This is the primary event handler that coordinates all state transitions and
/// side effects. It pattern-matches on event types, calls state mutation methods,
/// and collects actions to be executed by the plugin runtime.
///
/// # Parameters
///
/// * `state` - Mutable reference to application state
/// * `event` - Event to process
///
/// # Returns
///
/// A vector of actions to execute in sequence. May be empty if the event
/// requires no side effects (e.g., no project selected, state unchanged).
///
/// # Errors
///
/// Returns errors from state mutation methods or worker communication failures.
///
/// # Tracing
///
/// Each call creates an info-level span with the event type for debugging.
///
/// # Example
///
/// ```rust
/// use crate::app::{AppState, handler::{Event, handle_event}};
/// use crate::ui::theme::Theme;
///
/// let mut state = AppState::new(vec![], Theme::default());
/// let actions = handle_event(&mut state, &Event::KeyDown)?;
/// assert_eq!(actions.len(), 1); // Render action
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
pub fn handle_event(state: &mut AppState, event: &Event) -> Result<(bool, Vec<Action>)> {
    let _span = tracing::debug_span!("handle_event", event_type = ?event).entered();

    match event {
        Event::KeyDown => {
            state.move_selection_down();
            Ok((true, vec![]))
        }
        Event::KeyUp => {
            state.move_selection_up();
            Ok((true, vec![]))
        }
        Event::CloseFocus => Ok((false, vec![Action::CloseFocus])),
        Event::SelectProject => {
            use super::modes::InputMode;

            let Some(project) = state.selected_project() else {
                tracing::debug!("no project selected");
                if matches!(state.input_mode, InputMode::Search(_)) {
                    tracing::debug!("exiting search mode (no selection)");
                    state.input_mode = InputMode::Normal;
                    state.search_query = String::new();
                    state.apply_search_filter();
                    return Ok((true, vec![]));
                }
                return Ok((false, vec![]));
            };

            tracing::debug!(
                project_name = %project.name,
                project_path = %project.path,
                has_active_session = state.active_sessions.contains(&project.name),
                "project selected"
            );

            let mut actions = vec![];

            if state.active_sessions.contains(&project.name) {
                tracing::debug!(session_name = %project.name, "switching to existing session");
                actions.push(Action::SwitchSession {
                    name: project.name.clone(),
                    path: PathBuf::from(&project.path),
                    layout: project.layout.clone(),
                });
            } else {
                tracing::debug!(session_name = %project.name, "creating new session");
                actions.push(Action::CreateSession {
                    name: project.name.clone(),
                    path: PathBuf::from(&project.path),
                    layout: project.layout.clone(),
                });
            }

            Ok((false, actions))
        }
        Event::SearchMode => {
            use super::modes::{InputMode, SearchFocus};
            tracing::debug!("entering search mode");
            state.input_mode = InputMode::Search(SearchFocus::Typing);
            state.search_query = String::new();
            Ok((true, vec![]))
        }
        Event::FocusSearchBar => {
            use super::modes::{InputMode, SearchFocus};
            state.input_mode = InputMode::Search(SearchFocus::Typing);
            Ok((true, vec![]))
        }
        Event::FocusResults => {
            use super::modes::{InputMode, SearchFocus};

            if state.search_query.is_empty() {
                state.input_mode = InputMode::Normal;
                state.apply_search_filter();
                return Ok((true, vec![]));
            }

            state.input_mode = InputMode::Search(SearchFocus::Navigating);
            Ok((true, vec![]))
        }
        Event::ExitSearch => {
            use super::modes::InputMode;
            tracing::debug!(query = %state.search_query, "exiting search mode");
            state.input_mode = InputMode::Normal;
            state.search_query = String::new();
            state.apply_search_filter();
            Ok((true, vec![]))
        }
        Event::Char(c) => {
            use super::modes::InputMode;

            if !matches!(state.input_mode, InputMode::Search(_)) {
                return Ok((false, vec![]));
            }

            state.search_query.push(*c);

            tracing::trace!(query = %state.search_query, char = %c, "search query updated");

            state.apply_search_filter();

            Ok((true, vec![]))
        }
        Event::Backspace => {
            use super::modes::InputMode;
            if !matches!(state.input_mode, InputMode::Search(_)) {
                return Ok((false, vec![]));
            }

            state.search_query.pop();

            state.apply_search_filter();

            Ok((true, vec![]))
        }
        Event::Escape => {
            use super::modes::InputMode;
            state.input_mode = InputMode::Normal;

            state.search_query = String::new();

            state.apply_search_filter();

            Ok((true, vec![]))
        }
        Event::ShowProjects => {
            use super::modes::ViewMode;
            state.view_mode = ViewMode::ProjectsWithoutSessions;
            state.apply_search_filter();
            Ok((true, vec![]))
        }
        Event::ShowSessions => {
            use super::modes::ViewMode;
            state.view_mode = ViewMode::Sessions;
            state.apply_search_filter();
            Ok((true, vec![]))
        }
        Event::KillSession => {
            use super::modes::ViewMode;

            if state.view_mode != ViewMode::Sessions {
                return Ok((false, vec![]));
            }

            state.selected_project().map_or_else(|| {
                tracing::debug!("no session selected to kill");
                Ok((false, vec![]))
            }, |project| {
                tracing::debug!(session_name = %project.name, "killing session");
                Ok((false, vec![Action::KillSession {
                    name: project.name.clone()
                }]))
            })
        }
        Event::SessionUpdate { active_sessions, current_session } => {
            let mut actions = vec![];

            let added_count = active_sessions.difference(&state.active_sessions).count();
            let removed_count = state.active_sessions.difference(active_sessions).count();
            let current_changed = &state.current_session != current_session;

            tracing::debug!(
                total_sessions = active_sessions.len(),
                sessions_added = added_count,
                sessions_removed = removed_count,
                current_session = ?current_session,
                current_changed = current_changed,
                "session list updated"
            );

            if added_count > 0 || removed_count > 0 || current_changed {
                state.active_sessions.clone_from(active_sessions);
                state.current_session.clone_from(current_session);

                let session_names: Vec<String> = active_sessions.iter().cloned().collect();
                actions.push(Action::PostToWorker(
                    WorkerMessage::sync_sessions(session_names)
                ));

                state.apply_search_filter();
                Ok((true, actions))
            } else {
                tracing::debug!("sessions unchanged, skipping sync and render");
                Ok((false, actions))
            }
        }
        Event::ProjectsScanned { git_directories } => {
            tracing::debug!(
                projects_found = git_directories.len(),
                "projects scan completed"
            );

            // Extract project directories by stripping marker suffixes
            // (/.git or /.zessionizer) from the paths returned by find
            let projects: Vec<(String, String)> = git_directories
                .iter()
                .filter_map(|marker_path| {
                    // Determine if path starts with /host prefix to decide normalization strategy
                    let (without_host, is_sandbox_path) = if marker_path.starts_with("/host") {
                        (marker_path.strip_prefix("/host").unwrap_or(marker_path), true)
                    } else {
                        (marker_path.as_str(), false)
                    };

                    // Strip the marker suffixes (/.git or /.zessionizer)
                    let project_path = without_host
                        .strip_suffix("/.git")
                        .or_else(|| without_host.strip_suffix("/.zessionizer"))
                        .unwrap_or(without_host);

                    // Normalize the path by removing leading slashes if it's a relative path
                    // that was meant to be relative to home directory
                    let normalized_path = if project_path.starts_with("~/") {
                        project_path.to_string()
                    } else if project_path.starts_with('/') {
                        // Absolute path - keep as is
                        project_path.to_string()
                    } else if is_sandbox_path {
                        // Path was in sandbox format but became relative after stripping /host
                        format!("/{}", project_path)
                    } else {
                        project_path.to_string()
                    };

                    let project_name_raw = normalized_path
                        .split('/')
                        .next_back()
                        .unwrap_or("unknown");

                    let project_name = project_name_raw.to_string();

                    tracing::debug!(
                        project_name = %project_name,
                        project_path = %normalized_path,
                        original_path = %marker_path,
                        is_sandbox_path = is_sandbox_path,
                        "discovered project"
                    );

                    if project_name_raw != "unknown" {
                        Some((normalized_path, project_name))
                    } else {
                        tracing::debug!(path = %marker_path, "skipping invalid project path");
                        None
                    }
                })
                .collect();

            let mut actions = vec![];

            if projects.is_empty() {
                tracing::debug!("no new projects found during scan");
            } else {
                // Remove potential duplicates by using a unique key (project name maps to most recent path)
                use std::collections::HashMap;
                let mut unique_projects = HashMap::new();
                
                for (path, name) in projects {
                    // Only keep the first occurrence of each project name to avoid duplicates
                    unique_projects.entry(name.clone()).or_insert_with(|| (path, name));
                }
                
                let deduplicated_projects: Vec<(String, String)> = unique_projects.into_values().collect();
                
                tracing::debug!(
                    unique_projects_count = deduplicated_projects.len(),
                    original_count = git_directories.len(),
                    "deduplicated projects"
                );

                actions.push(Action::PostToWorker(
                    WorkerMessage::add_projects_batch(deduplicated_projects)
                ));
            }

            Ok((false, actions))
        }
        Event::ScanFailed { error } => {
            tracing::debug!(error = %error, "project scan failed");
            Ok((false, vec![]))
        }
        Event::PermissionsResult { granted: _ } => {
            Ok((false, vec![]))
        }
        Event::WorkerResponse(response) => {
            match response {
                WorkerResponse::ProjectsLoaded { projects } => {
                    if &state.projects == projects {
                        tracing::debug!("projects unchanged, skipping render");
                        Ok((false, vec![]))
                    } else {
                        let old_filtered = state.filtered_projects.clone();
                        state.projects.clone_from(projects);
                        state.apply_search_filter();

                        if state.filtered_projects == old_filtered {
                            tracing::debug!("filtered projects unchanged after reload, skipping render");
                            Ok((false, vec![]))
                        } else {
                            Ok((true, vec![]))
                        }
                    }
                }
                WorkerResponse::FrecencyUpdated { path: _ } | WorkerResponse::SessionsSynced { count: _ } => {
                    Ok((false, vec![]))
                }
                WorkerResponse::ProjectsBatchAdded { count, projects } => {
                    tracing::debug!(count = count, "projects batch added successfully");
                    if &state.projects == projects {
                        tracing::debug!("projects unchanged after batch add, skipping render");
                        Ok((false, vec![]))
                    } else {
                        let old_filtered = state.filtered_projects.clone();
                        state.projects.clone_from(projects);
                        state.apply_search_filter();

                        if state.filtered_projects == old_filtered {
                            tracing::debug!("filtered projects unchanged after batch add, skipping render");
                            Ok((false, vec![]))
                        } else {
                            Ok((true, vec![]))
                        }
                    }
                }
                WorkerResponse::LayoutUpdated { path: _ } => {
                    tracing::debug!("project layout updated successfully");
                    // Refresh projects to reflect the layout change
                    Ok((false, vec![Action::PostToWorker(WorkerMessage::load_projects(false))]))
                }
                WorkerResponse::Error { message } => {
                    tracing::error!("Worker error: {}", message);
                    Ok((true, vec![]))
                }
            }
        }
        Event::UpdateProjectLayout { path, layout } => {
            tracing::debug!(project_path = %path, layout = ?layout, "handling update project layout event");
            Ok((false, vec![Action::UpdateProjectLayout {
                path: path.clone(),
                layout: layout.clone(),
            }]))
        }
    }
}
