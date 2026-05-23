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

use crate::app::state::WorktreesContext;
use crate::app::{Action, AppState};
use crate::domain::error::Result;
use crate::infrastructure::{from_sandbox_path, worktree as worktree_scan};
use crate::worker::{WorkerMessage, WorkerResponse};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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
    /// Returns from the worktrees drilldown to the previous projects view.
    Back,

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
            use super::modes::{InputMode, ViewMode};

            if matches!(state.view_mode, ViewMode::Worktrees) {
                let Some(worktree) = state.selected_worktree() else {
                    return Ok((false, vec![]));
                };
                let Some(ctx) = state.worktrees_context.as_ref() else {
                    return Ok((false, vec![]));
                };

                let session_name = ctx.project_name.clone();
                let project_path = ctx.project_path.clone();
                let cwd = PathBuf::from(&worktree.path);

                tracing::debug!(
                    session = %session_name,
                    worktree = %worktree.name,
                    path = %worktree.path,
                    "opening session at worktree"
                );

                // Record against the project root, not the worktree -- the
                // Sessions view filters projects by their own `path`, not by
                // the cwd a particular worktree was opened with.
                state
                    .session_paths
                    .insert(session_name.clone(), project_path);

                let action = if state.active_sessions.contains(&session_name) {
                    Action::SwitchSession {
                        name: session_name,
                        path: cwd,
                    }
                } else {
                    Action::CreateSession {
                        name: session_name,
                        path: cwd,
                    }
                };

                return Ok((false, vec![action]));
            }

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

            // `scan` always returns at least the trunk entry; sibling
            // worktrees mean `len() > 1`. Calling it once instead of pairing
            // with `has_worktrees_dir` halves the syscalls on every Enter.
            let worktrees = worktree_scan::scan(&project.path);
            if worktrees.len() > 1 {
                tracing::debug!(project = %project.name, "drilling into worktrees view");
                let project_name = project.name.clone();
                let project_path = project.path.clone();
                let previous_view = state.view_mode;

                state.worktrees = worktrees;
                state.worktrees_context = Some(WorktreesContext {
                    project_name,
                    project_path,
                    previous_view,
                });
                state.view_mode = ViewMode::Worktrees;
                state.search_query = String::new();
                state.input_mode = InputMode::Normal;
                state.selected_index = 0;
                state.apply_search_filter();
                return Ok((true, vec![]));
            }

            let mut actions = vec![];

            let project_name = project.name.clone();
            let project_path = project.path.clone();

            state
                .session_paths
                .insert(project_name.clone(), project_path.clone());

            if state.active_sessions.contains(&project_name) {
                tracing::debug!(session_name = %project_name, "switching to existing session");
                actions.push(Action::SwitchSession {
                    name: project_name,
                    path: PathBuf::from(project_path),
                });
            } else {
                tracing::debug!(session_name = %project_name, "creating new session");
                actions.push(Action::CreateSession {
                    name: project_name,
                    path: PathBuf::from(project_path),
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
        Event::Back => {
            use super::modes::ViewMode;
            if !matches!(state.view_mode, ViewMode::Worktrees) {
                return Ok((false, vec![]));
            }
            let previous = state
                .worktrees_context
                .as_ref()
                .map_or(ViewMode::Sessions, |c| c.previous_view);
            state.view_mode = previous;
            state.worktrees_context = None;
            state.worktrees = vec![];
            state.filtered_worktrees = vec![];
            state.search_query = String::new();
            state.selected_index = 0;
            state.apply_search_filter();
            Ok((true, vec![]))
        }
        Event::KillSession => {
            use super::modes::ViewMode;

            if state.view_mode != ViewMode::Sessions {
                return Ok((false, vec![]));
            }

            let Some(name) = state.selected_project().map(|p| p.name.clone()) else {
                tracing::debug!("no session selected to kill");
                return Ok((false, vec![]));
            };

            tracing::debug!(session_name = %name, "killing session");
            // Drop the path record now -- a future poll's `retain` would do
            // the same, but a stale entry between here and the next
            // `SessionUpdate` would mismatch any same-named project the
            // user opens in that window.
            state.session_paths.remove(&name);

            Ok((false, vec![Action::KillSession { name }]))
        }
        Event::SessionUpdate {
            active_sessions,
            current_session,
        } => {
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
                // Drop path records for sessions Zellij no longer reports as
                // alive, so a future open of an unrelated project that
                // happens to reuse the same name doesn't inherit a stale
                // path-mismatch from a dead session.
                state
                    .session_paths
                    .retain(|name, _| active_sessions.contains(name));

                let session_names: Vec<String> = active_sessions.iter().cloned().collect();
                actions.push(Action::PostToWorker(WorkerMessage::sync_sessions(
                    session_names,
                )));

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

            // Strip the marker suffix (`/.git` or `/.zessionizer`) and convert
            // the in-sandbox `/host/...` form back to the user-visible `~/...`
            // form. Two markers in the same project (e.g. a repo that also has
            // a `.zessionizer` file) collapse to a single entry via dedup, and
            // a project nested inside another (e.g. a Git submodule whose own
            // `.git` was matched) is dropped in favor of its outermost
            // ancestor.
            let mut candidates: Vec<(String, String)> = Vec::with_capacity(git_directories.len());
            let mut seen: HashSet<String> = HashSet::new();

            for marker_path in git_directories {
                let project_sandbox = marker_path
                    .strip_suffix("/.git")
                    .or_else(|| marker_path.strip_suffix("/.zessionizer"))
                    .unwrap_or(marker_path);

                let project_path = from_sandbox_path(project_sandbox);

                if !seen.insert(project_path.clone()) {
                    continue;
                }

                let project_name = project_path
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("unknown")
                    .to_string();

                candidates.push((project_path, project_name));
            }

            // Sort shortest-first so any accepted entry is a candidate
            // ancestor for the longer paths that follow.
            candidates.sort_by_key(|(p, _)| p.len());

            let mut projects: Vec<(String, String)> = Vec::with_capacity(candidates.len());
            for (project_path, project_name) in candidates {
                let nested = projects.iter().any(|(parent, _)| is_descendant(&project_path, parent));
                if nested {
                    tracing::debug!(
                        project_path = %project_path,
                        "skipping nested project (ancestor already accepted)"
                    );
                    continue;
                }

                tracing::debug!(
                    project_name = %project_name,
                    project_path = %project_path,
                    "discovered project"
                );

                projects.push((project_path, project_name));
            }

            let mut actions = vec![];

            if projects.is_empty() {
                tracing::debug!("no new projects found during scan");
            } else {
                actions.push(Action::PostToWorker(WorkerMessage::add_projects_batch(
                    projects,
                )));
            }

            Ok((false, actions))
        }
        Event::ScanFailed { error } => {
            tracing::debug!(error = %error, "project scan failed");
            Ok((false, vec![]))
        }
        Event::PermissionsResult { granted: _ } => Ok((false, vec![])),
        Event::WorkerResponse(response) => match response {
            WorkerResponse::ProjectsLoaded { projects } => {
                if &state.projects == projects {
                    tracing::debug!("projects unchanged, skipping render");
                    Ok((false, vec![]))
                } else {
                    let old_filtered = state.filtered_projects.clone();
                    state.projects.clone_from(projects);
                    state.apply_search_filter();

                    if state.filtered_projects == old_filtered {
                        tracing::debug!(
                            "filtered projects unchanged after reload, skipping render"
                        );
                        Ok((false, vec![]))
                    } else {
                        Ok((true, vec![]))
                    }
                }
            }
            WorkerResponse::FrecencyUpdated { path: _ }
            | WorkerResponse::SessionsSynced { count: _ } => Ok((false, vec![])),
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
                        tracing::debug!(
                            "filtered projects unchanged after batch add, skipping render"
                        );
                        Ok((false, vec![]))
                    } else {
                        Ok((true, vec![]))
                    }
                }
            }
            WorkerResponse::Error { message } => {
                tracing::error!("Worker error: {}", message);
                Ok((true, vec![]))
            }
        },
    }
}

/// Returns `true` if `candidate` sits inside `parent` (and isn't `parent`
/// itself). `Path::starts_with` matches whole components, so `~/a/foo` is
/// not falsely treated as a descendant of `~/a/foobar`.
fn is_descendant(candidate: &str, parent: &str) -> bool {
    let parent_path = Path::new(parent.trim_end_matches('/'));
    let candidate_path = Path::new(candidate);
    candidate_path != parent_path && candidate_path.starts_with(parent_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::modes::ViewMode;
    use crate::domain::Project;
    use crate::ui::theme::Theme;
    use crate::worker::WorkerMessage;
    use std::fs;
    use tempfile::TempDir;

    fn extract_batch(actions: &[Action]) -> &[(String, String)] {
        match actions {
            [Action::PostToWorker(WorkerMessage::AddProjectsBatch { projects, .. })] => projects,
            other => panic!("expected single AddProjectsBatch action, got {other:?}"),
        }
    }

    fn empty_state() -> AppState {
        AppState::new(vec![], Theme::default())
    }

    fn scan_event(markers: &[&str]) -> Event {
        Event::ProjectsScanned {
            git_directories: markers.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    fn project_at(path: &str) -> Project {
        Project::new(path.to_string(), "demo".to_string())
    }

    fn state_with_project(path: &str) -> AppState {
        let mut state = AppState::new(vec![project_at(path)], Theme::default());
        state.view_mode = ViewMode::ProjectsWithoutSessions;
        state.apply_search_filter();
        state
    }

    #[test]
    fn projects_scanned_strips_marker_and_uses_user_paths() {
        let mut state = empty_state();
        let event = scan_event(&["/host/Projects/foo/.git"]);

        let (_, actions) = handle_event(&mut state, &event).unwrap();
        let projects = extract_batch(&actions);

        assert_eq!(
            projects,
            &[("~/Projects/foo".to_string(), "foo".to_string())]
        );
    }

    #[test]
    fn projects_scanned_dedups_repos_with_both_markers() {
        let mut state = empty_state();
        let event = scan_event(&[
            "/host/Projects/foo/.git",
            "/host/Projects/foo/.zessionizer",
        ]);

        let (_, actions) = handle_event(&mut state, &event).unwrap();
        let projects = extract_batch(&actions);

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].0, "~/Projects/foo");
    }

    #[test]
    fn projects_scanned_keeps_distinct_paths_with_same_basename() {
        let mut state = empty_state();
        let event = scan_event(&["/host/Projects/foo/.git", "/host/Code/foo/.git"]);

        let (_, actions) = handle_event(&mut state, &event).unwrap();
        let projects = extract_batch(&actions);

        assert_eq!(projects.len(), 2);
        let paths: Vec<&str> = projects.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"~/Projects/foo"));
        assert!(paths.contains(&"~/Code/foo"));
    }

    #[test]
    fn projects_scanned_drops_nested_submodule_in_favor_of_parent() {
        // `find` returns both the outer repo's `.git` and a submodule's
        // `.git` deep inside it. Only the outer repo should be surfaced;
        // the nested entry would create a phantom row that collides with
        // any unrelated standalone checkout sharing the same basename.
        let mut state = empty_state();
        let event = scan_event(&[
            "/host/parent/outer/.git",
            "/host/parent/outer/inner/.git",
        ]);

        let (_, actions) = handle_event(&mut state, &event).unwrap();
        let projects = extract_batch(&actions);

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].0, "~/parent/outer");
    }

    #[test]
    fn projects_scanned_does_not_drop_unrelated_paths_with_shared_prefix() {
        // `~/parent/foo` must not be treated as a descendant of
        // `~/parent/foobar` -- the prefix coincides at a non-boundary
        // character.
        let mut state = empty_state();
        let event = scan_event(&[
            "/host/parent/foobar/.git",
            "/host/parent/foo/.git",
        ]);

        let (_, actions) = handle_event(&mut state, &event).unwrap();
        let projects = extract_batch(&actions);

        let paths: Vec<&str> = projects.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(projects.len(), 2);
        assert!(paths.contains(&"~/parent/foo"));
        assert!(paths.contains(&"~/parent/foobar"));
    }

    #[test]
    fn projects_scanned_emits_no_action_when_empty() {
        let mut state = empty_state();
        let event = scan_event(&[]);

        let (_, actions) = handle_event(&mut state, &event).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn select_project_drills_into_worktrees_when_dir_exists() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".worktrees")).unwrap();
        fs::create_dir(dir.path().join(".worktrees").join("feat-a")).unwrap();

        let mut state = state_with_project(dir.path().to_str().unwrap());

        let (rendered, actions) = handle_event(&mut state, &Event::SelectProject).unwrap();

        assert!(
            rendered,
            "drilling into worktrees should request a re-render"
        );
        assert!(
            actions.is_empty(),
            "drilldown should not emit a session action"
        );
        assert_eq!(state.view_mode, ViewMode::Worktrees);
        let names: Vec<&str> = state
            .filtered_worktrees
            .iter()
            .map(|w| w.name.as_str())
            .collect();
        assert_eq!(names, vec!["trunk", "feat-a"]);

        let ctx = state.worktrees_context.as_ref().expect("context set");
        assert_eq!(ctx.previous_view, ViewMode::ProjectsWithoutSessions);
        assert_eq!(ctx.project_name, "demo");
    }

    #[test]
    fn select_project_opens_session_when_no_worktrees_dir() {
        let dir = TempDir::new().unwrap();
        let mut state = state_with_project(dir.path().to_str().unwrap());

        let (_, actions) = handle_event(&mut state, &Event::SelectProject).unwrap();

        assert_eq!(state.view_mode, ViewMode::ProjectsWithoutSessions);
        assert!(matches!(actions.as_slice(), [Action::CreateSession { .. }]));
    }

    #[test]
    fn select_worktree_emits_session_action_with_worktree_path() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".worktrees")).unwrap();
        let wt_dir = dir.path().join(".worktrees").join("feat-a");
        fs::create_dir(&wt_dir).unwrap();

        let mut state = state_with_project(dir.path().to_str().unwrap());
        handle_event(&mut state, &Event::SelectProject).unwrap();
        // Move selection to feat-a (trunk is index 0).
        state.selected_index = 1;

        let (_, actions) = handle_event(&mut state, &Event::SelectProject).unwrap();

        match actions.as_slice() {
            [Action::CreateSession { name, path, .. }] => {
                assert_eq!(name, "demo", "session name should be the project name");
                assert_eq!(path, &wt_dir, "cwd should be the worktree path");
            }
            other => panic!("expected CreateSession, got {other:?}"),
        }
    }

    #[test]
    fn back_returns_to_previous_view_and_clears_worktrees() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".worktrees")).unwrap();
        fs::create_dir(dir.path().join(".worktrees").join("feat-a")).unwrap();

        let mut state = state_with_project(dir.path().to_str().unwrap());
        handle_event(&mut state, &Event::SelectProject).unwrap();
        assert_eq!(state.view_mode, ViewMode::Worktrees);

        let (rendered, actions) = handle_event(&mut state, &Event::Back).unwrap();

        assert!(rendered);
        assert!(actions.is_empty());
        assert_eq!(state.view_mode, ViewMode::ProjectsWithoutSessions);
        assert!(state.worktrees.is_empty());
        assert!(state.filtered_worktrees.is_empty());
        assert!(state.worktrees_context.is_none());
    }

    #[test]
    fn back_is_noop_when_not_in_worktrees_view() {
        let dir = TempDir::new().unwrap();
        let mut state = state_with_project(dir.path().to_str().unwrap());

        let (rendered, actions) = handle_event(&mut state, &Event::Back).unwrap();

        assert!(!rendered);
        assert!(actions.is_empty());
        assert_eq!(state.view_mode, ViewMode::ProjectsWithoutSessions);
    }

    #[test]
    fn select_project_records_session_path_for_disambiguation() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap().to_string();
        let mut state = state_with_project(&path);

        handle_event(&mut state, &Event::SelectProject).unwrap();

        assert_eq!(state.session_paths.get("demo"), Some(&path));
    }

    #[test]
    fn project_with_active_session_at_other_path_is_not_active() {
        // Two projects share the basename "demo" -- one at /a/demo, one at
        // /b/demo. A session named "demo" was opened against /a/demo. Only
        // /a/demo should report as having an active session.
        let theme = Theme::default();
        let project_a = Project::new("/a/demo".to_string(), "demo".to_string());
        let project_b = Project::new("/b/demo".to_string(), "demo".to_string());
        let mut state = AppState::new(vec![project_a.clone(), project_b.clone()], theme);
        state
            .active_sessions
            .insert("demo".to_string());
        state
            .session_paths
            .insert("demo".to_string(), "/a/demo".to_string());

        assert!(state.project_has_active_session(&project_a));
        assert!(!state.project_has_active_session(&project_b));
    }

    #[test]
    fn unknown_session_falls_back_to_name_match() {
        // Session that wasn't opened by zessionizer this lifetime -- we have
        // no path record. Fall back to legacy name-only matching so the row
        // still appears somewhere.
        let theme = Theme::default();
        let project = Project::new("/some/demo".to_string(), "demo".to_string());
        let mut state = AppState::new(vec![project.clone()], theme);
        state.active_sessions.insert("demo".to_string());

        assert!(state.project_has_active_session(&project));
    }

    #[test]
    fn session_update_prunes_paths_for_dead_sessions() {
        // A session named "demo" was opened against /a/demo, then killed.
        // The next SessionUpdate should drop the stale path so a different
        // project that later opens with the same name doesn't inherit a
        // path-mismatch from the dead one.
        let dir = TempDir::new().unwrap();
        let mut state = state_with_project(dir.path().to_str().unwrap());
        state
            .session_paths
            .insert("demo".to_string(), "/a/demo".to_string());
        state.active_sessions.insert("demo".to_string());

        let event = Event::SessionUpdate {
            active_sessions: HashSet::new(),
            current_session: None,
        };
        handle_event(&mut state, &event).unwrap();

        assert!(state.session_paths.is_empty());
    }

    #[test]
    fn kill_session_drops_path_record_immediately() {
        // The user kills the selected session via `K`. The path record
        // should be removed in the same turn -- not on the next
        // SessionUpdate poll -- so a same-named project opened before the
        // poll fires doesn't see a stale path mismatch.
        let theme = Theme::default();
        let project = Project::new("/a/demo".to_string(), "demo".to_string());
        let mut state = AppState::new(vec![project], theme);
        state.view_mode = ViewMode::Sessions;
        state.active_sessions.insert("demo".to_string());
        state
            .session_paths
            .insert("demo".to_string(), "/a/demo".to_string());
        state.apply_search_filter();

        let (_, actions) = handle_event(&mut state, &Event::KillSession).unwrap();

        assert!(matches!(
            actions.as_slice(),
            [Action::KillSession { name }] if name == "demo"
        ));
        assert!(!state.session_paths.contains_key("demo"));
    }
}
