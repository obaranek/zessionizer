//! Input and view mode state types for the application.
//!
//! This module defines the state machine enums that control user interaction
//! modes and view filtering. These types determine which keybindings are active,
//! how input is processed, and which projects are displayed.
//!
//! # State Machine
//!
//! The application operates in one of two primary input modes:
//! - **Normal**: Default navigation and command mode
//! - **Search**: Active search with typing or result navigation focus
//!
//! View modes control which projects are visible:
//! - **Sessions**: Projects with active Zellij sessions
//! - **`ProjectsWithoutSessions`**: All projects without active sessions
//!
//! # Example
//!
//! ```rust
//! use crate::app::modes::{InputMode, SearchFocus, ViewMode};
//!
//! let input_mode = InputMode::Search(SearchFocus::Typing);
//! let view_mode = ViewMode::Sessions;
//! ```

/// Focus state within search mode.
///
/// Determines whether search input is being typed or search results are being
/// navigated. Controls which keybindings are active during search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFocus {
    /// User is typing in the search input field.
    ///
    /// Accepts character input, backspace, and enter (to switch to Navigating).
    Typing,

    /// User is navigating through filtered search results.
    ///
    /// Accepts j/k for movement, enter to select, and / to return to Typing.
    Navigating,
}

/// Current input handling mode.
///
/// Controls which keybindings are active and how user input is processed.
/// Determines the displayed footer text and available commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Default navigation and command mode.
    ///
    /// Available keybindings: j/k (navigate), / (search), enter (select),
    /// K (kill session), n (new project view), s (sessions view), q (quit).
    Normal,

    /// Active search mode with focus state.
    ///
    /// Contains a [`SearchFocus`] variant indicating whether the user is typing
    /// or navigating results. Footer displays search-specific keybindings.
    Search(SearchFocus),
}

/// View filtering mode determining which projects are displayed.
///
/// Controls the base set of projects before search filtering is applied.
/// Changes the header title and available actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Shows only projects with active Zellij sessions.
    ///
    /// Header displays "Active Sessions". Available actions: switch, kill.
    Sessions,

    /// Shows all projects without active sessions.
    ///
    /// Header displays "All Projects". Available actions: create session.
    ProjectsWithoutSessions,
}
