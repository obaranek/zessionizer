//! View model types representing renderable UI state.
//!
//! This module defines immutable view models computed from application state,
//! following the MVVM pattern. View models are optimized for rendering and
//! contain pre-computed display information like highlight ranges and selection
//! state.
//!
//! # Architecture
//!
//! View models are created via `AppState::compute_viewmodel()` and consumed by
//! the renderer. They contain no business logic, only display-ready data.
//!
//! # Example
//!
//! ```rust
//! use crate::ui::viewmodel::{UIViewModel, DisplayItem};
//!
//! let vm = UIViewModel {
//!     display_items: vec![DisplayItem {
//!         name: "my-project".to_string(),
//!         path: "/home/user/code/my-project".to_string(),
//!         is_selected: true,
//!         highlight_ranges: vec![(0, 2)],
//!     }],
//!     selected_index: 0,
//!     header: HeaderInfo { title: "Zessionizer".to_string() },
//!     footer: FooterInfo { keybindings: "q: quit".to_string() },
//!     empty_state: None,
//!     search_bar: None,
//! };
//! ```

/// Complete UI view model for rendering.
///
/// Contains all display information needed to render the plugin UI. The view
/// model is computed from `AppState` and includes pre-processed display items,
/// selection state, and optional UI elements like search bars and empty states.
#[derive(Debug, Clone)]
pub struct UIViewModel {
    /// List of items to display in the table.
    pub display_items: Vec<DisplayItem>,

    /// Index of the currently selected item.
    pub selected_index: usize,

    /// Header information (title, branding).
    pub header: HeaderInfo,

    /// Footer information (keybindings, help text).
    pub footer: FooterInfo,

    /// Optional empty state message (when no items are available).
    pub empty_state: Option<EmptyState>,

    /// Optional search bar information (when in search mode).
    pub search_bar: Option<SearchBarInfo>,
}

/// Display information for a single project or session item.
///
/// Represents one row in the table view. Contains pre-computed highlight ranges
/// for fuzzy match rendering.
#[derive(Debug, Clone)]
pub struct DisplayItem {
    /// Display name (project name or session name).
    pub name: String,

    /// Full path or identifier.
    pub path: String,

    /// Whether this item is currently selected.
    pub is_selected: bool,

    /// Whether this is the current active session.
    pub is_current_session: bool,

    /// Character ranges to highlight (for fuzzy search matches).
    ///
    /// Each tuple is `(start_index, end_index)` in UTF-8 character indices.
    pub highlight_ranges: Vec<(usize, usize)>,
}

/// Header display information.
///
/// Contains title and branding information for the top of the UI.
#[derive(Debug, Clone)]
pub struct HeaderInfo {
    /// Title text to display in the header.
    pub title: String,
}

/// Footer display information.
///
/// Contains help text and keybinding hints for the bottom of the UI.
#[derive(Debug, Clone)]
pub struct FooterInfo {
    /// Keybinding help text (e.g., "q: quit | /: search | n: projects").
    pub keybindings: String,
}

/// Empty state message display information.
///
/// Shown when no items are available (e.g., no projects found, no sessions).
#[derive(Debug, Clone)]
pub struct EmptyState {
    /// Primary message (e.g., "No projects found").
    pub message: String,

    /// Secondary explanatory text (e.g., "Add projects to get started").
    pub subtitle: String,
}

/// Search bar display information.
///
/// Contains the current search query for rendering the search input box.
#[derive(Debug, Clone)]
pub struct SearchBarInfo {
    /// Current search query text.
    pub query: String,
}
