//! Application state management and view model computation.
//!
//! This module defines [`AppState`], the central state container for the plugin,
//! along with methods for filtering, selection management, and UI view model
//! generation. It serves as the single source of truth for all transient UI state.
//!
//! # Architecture
//!
//! `AppState` separates core data (projects, active sessions) from derived state
//! (filtered projects, selected index) to maintain consistency and simplify
//! state transitions. View models are computed on-demand from state snapshots.
//!
//! # State Components
//!
//! - **Projects**: Master list of all known projects from storage
//! - **Filtered Projects**: Subset after applying view mode and search filters
//! - **Selection**: Current cursor position within filtered results
//! - **Input Mode**: Controls keybinding interpretation and UI layout
//! - **View Mode**: Determines base project filtering (sessions vs. all)
//! - **Active Sessions**: Set of currently running Zellij session names
//!
//! # View Model Computation
//!
//! The `compute_viewmodel` method transforms state into a renderable UI
//! representation, handling windowing, fuzzy match highlighting, and responsive
//! layout adjustments based on terminal dimensions.
//!
//! # Example
//!
//! ```rust
//! use crate::app::AppState;
//! use crate::domain::Project;
//! use crate::ui::theme::Theme;
//!
//! let projects = vec![
//!     Project { name: "project-a".into(), path: "/path/to/a".into() },
//! ];
//! let mut state = AppState::new(projects, Theme::default());
//! state.apply_search_filter();
//! let viewmodel = state.compute_viewmodel(24, 80);
//! ```

use crate::domain::Project;
use crate::ui::theme::Theme;
use super::modes::{InputMode, ViewMode};
use std::collections::HashSet;
use fuzzy_matcher::skim::SkimMatcherV2;

/// Central application state container.
///
/// Holds all transient UI state including project lists, filters, selection,
/// and mode information. Mutated by the event handler in response to user input
/// and system events. View models are computed on-demand from state snapshots.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Master list of all projects loaded from storage.
    ///
    /// Sorted by frecency score (most recent/frequent first). Updated when
    /// worker responses arrive with new or reordered projects.
    pub projects: Vec<Project>,

    /// Projects matching current view mode and search query.
    ///
    /// Recomputed by `apply_search_filter()` after state changes. Used for
    /// rendering and selection bounds checking.
    pub filtered_projects: Vec<Project>,

    /// Zero-based index of selected project within `filtered_projects`.
    ///
    /// Clamped to valid bounds by `apply_search_filter()`. Wraps around during
    /// navigation via `move_selection_up/down()`.
    pub selected_index: usize,

    /// Current input handling mode.
    ///
    /// Determines active keybindings and UI layout (search bar visibility,
    /// footer text). Changed by mode switching events.
    pub input_mode: InputMode,

    /// Current search query string.
    ///
    /// Accumulated by `Char` events, reduced by `Backspace` events, cleared
    /// by `ExitSearch` and `Escape` events. Tokenized for filtering.
    pub search_query: String,

    /// Current view filtering mode.
    ///
    /// Determines which projects are visible before search filtering. Changed
    /// by `ShowProjects` and `ShowSessions` events.
    pub view_mode: ViewMode,

    /// Color scheme for UI rendering.
    ///
    /// Loaded from Zellij configuration on plugin initialization. Stored in
    /// state for access by view model computation.
    pub theme: Theme,

    /// Set of currently active Zellij session names.
    ///
    /// Updated by `SessionUpdate` events. Used for view mode filtering and
    /// determining whether to create or switch sessions.
    pub active_sessions: HashSet<String>,

    /// Name of the current Zellij session.
    ///
    /// Updated by `SessionUpdate` events. Used to filter out the current session
    /// from the Sessions view.
    pub current_session: Option<String>,
}

impl AppState {
    /// Creates a new application state with initial projects and theme.
    ///
    /// Initializes all collections to empty, sets default modes (Normal input,
    /// Sessions view), and prepares for project filtering.
    ///
    /// # Parameters
    ///
    /// * `projects` - Initial project list (typically empty until worker loads data)
    /// * `theme` - Color scheme for UI rendering
    ///
    /// # Returns
    ///
    /// A new `AppState` with default initialization.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::app::AppState;
    /// use crate::ui::theme::Theme;
    ///
    /// let state = AppState::new(vec![], Theme::default());
    /// assert_eq!(state.selected_index, 0);
    /// ```
    #[must_use]
    pub fn new(projects: Vec<Project>, theme: Theme) -> Self {
        Self {
            projects,
            filtered_projects: vec![],
            selected_index: 0,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            view_mode: ViewMode::Sessions,
            theme,
            active_sessions: HashSet::new(),
            current_session: None,
        }
    }

    /// Moves selection cursor down by one position, wrapping to top if at end.
    ///
    /// Called by `KeyDown` event handler. No-op if filtered projects list is empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::app::AppState;
    /// # use crate::ui::theme::Theme;
    /// # let mut state = AppState::new(vec![], Theme::default());
    /// state.move_selection_down();
    /// ```
    pub fn move_selection_down(&mut self) {
        if self.filtered_projects.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.filtered_projects.len();
    }

    /// Moves selection cursor up by one position, wrapping to bottom if at start.
    ///
    /// Called by `KeyUp` event handler. No-op if filtered projects list is empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::app::AppState;
    /// # use crate::ui::theme::Theme;
    /// # let mut state = AppState::new(vec![], Theme::default());
    /// state.move_selection_up();
    /// ```
    pub fn move_selection_up(&mut self) {
        if self.filtered_projects.is_empty() {
            return;
        }
        if self.selected_index == 0 {
            self.selected_index = self.filtered_projects.len() - 1;
        } else {
            self.selected_index -= 1;
        }
    }

    /// Returns a reference to the currently selected project, if any.
    ///
    /// Returns `None` if the filtered projects list is empty or the selected index
    /// is out of bounds (which should never occur due to clamping).
    ///
    /// # Returns
    ///
    /// - `Some(&Project)` if a project is selected
    /// - `None` if no projects are visible
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::app::AppState;
    /// # use crate::ui::theme::Theme;
    /// # let mut state = AppState::new(vec![], Theme::default());
    /// if let Some(project) = state.selected_project() {
    ///     println!("Selected: {}", project.name);
    /// }
    /// ```
    #[must_use]
    pub fn selected_project(&self) -> Option<&Project> {
        self.filtered_projects.get(self.selected_index)
    }

    /// Applies view mode and search filters to the master project list.
    ///
    /// First filters by view mode (sessions vs. all projects), then applies
    /// multi-token search query filtering. Updates `filtered_projects` and clamps
    /// `selected_index` to valid bounds.
    ///
    /// # Filtering Algorithm
    ///
    /// 1. **View Mode Filter**: Include only projects with/without active sessions
    /// 2. **Search Query Tokenization**: Split query by whitespace, lowercase
    /// 3. **Token Matching**: Require all tokens to appear in project name (substring)
    /// 4. **Index Clamping**: Adjust selection to remain within bounds
    ///
    /// # Tracing
    ///
    /// Creates a debug-level span with total projects, query length, and view mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::app::AppState;
    /// # use crate::ui::theme::Theme;
    /// # let mut state = AppState::new(vec![], Theme::default());
    /// state.search_query = "my-proj".to_string();
    /// state.apply_search_filter();
    /// ```
    pub fn apply_search_filter(&mut self) {
        use fuzzy_matcher::FuzzyMatcher;

        let _span = tracing::debug_span!("apply_search_filter",
            total_projects = self.projects.len(),
            query_len = self.search_query.len(),
            view_mode = ?self.view_mode
        ).entered();

        let tokens: Vec<String> = if self.search_query.is_empty() {
            vec![]
        } else {
            self.search_query
                .split_whitespace()
                .map(str::to_lowercase)
                .collect()
        };

        let matcher = if tokens.is_empty() {
            None
        } else {
            Some(SkimMatcherV2::default())
        };

        let filtered_iter = self.projects.iter().filter(|project| {
            let passes_view_mode = match self.view_mode {
                ViewMode::Sessions => self.active_sessions.contains(&project.name),
                ViewMode::ProjectsWithoutSessions => !self.active_sessions.contains(&project.name),
            };

            if !passes_view_mode {
                return false;
            }

            matcher.as_ref().map_or(true, |m| {
                let name_lower = project.name.to_lowercase();
                tokens.iter().all(|token| m.fuzzy_match(&name_lower, token).is_some())
            })
        });

        self.filtered_projects = filtered_iter.cloned().collect();

        if self.filtered_projects.is_empty() {
            self.selected_index = 0;
        } else {
            self.selected_index = self.selected_index.min(self.filtered_projects.len() - 1);
        }

        tracing::debug!(
            filtered_count = self.filtered_projects.len(),
            "search filter applied"
        );
    }

    /// Computes a renderable UI view model from current state and terminal dimensions.
    ///
    /// Transforms application state into a structured representation optimized for
    /// rendering. Handles windowing (showing subset of results), fuzzy match
    /// highlighting, responsive path truncation, and empty state handling.
    ///
    /// # Parameters
    ///
    /// * `rows` - Terminal height in character cells
    /// * `cols` - Terminal width in character cells
    ///
    /// # Returns
    ///
    /// A [`UIViewModel`](crate::ui::viewmodel::UIViewModel) containing display items,
    /// header/footer info, search bar state, and optional empty state message.
    ///
    /// # Windowing Algorithm
    ///
    /// 1. Calculate available rows after subtracting UI chrome (header, footer, search)
    /// 2. Center window around selected index (selected index at midpoint)
    /// 3. Adjust window if near start/end to maximize visible items
    /// 4. Compute relative selection index within visible window
    ///
    /// # Example
    ///
    /// ```rust
    /// # use crate::app::AppState;
    /// # use crate::ui::theme::Theme;
    /// # let state = AppState::new(vec![], Theme::default());
    /// let viewmodel = state.compute_viewmodel(24, 80);
    /// ```
    #[must_use]
    pub fn compute_viewmodel(&self, rows: usize, cols: usize) -> crate::ui::viewmodel::UIViewModel {
        if self.projects.is_empty() || self.filtered_projects.is_empty() {
            return crate::ui::viewmodel::UIViewModel {
                display_items: vec![],
                selected_index: 0,
                header: self.compute_header(),
                footer: self.compute_footer(),
                empty_state: None,
                search_bar: self.compute_search_bar(),
            };
        }

        let available_rows = self.calculate_available_rows(rows);

        let mut visible_start = self.selected_index.saturating_sub(available_rows / 2);
        let visible_end = (visible_start + available_rows).min(self.filtered_projects.len());

        let actual_count = visible_end - visible_start;
        if actual_count < available_rows && self.filtered_projects.len() >= available_rows {
            visible_start = visible_end.saturating_sub(available_rows);
        }

        let matcher = if matches!(self.input_mode, InputMode::Search(_)) && !self.search_query.is_empty() {
            Some(SkimMatcherV2::default())
        } else {
            None
        };

        let display_items: Vec<crate::ui::viewmodel::DisplayItem> = self.filtered_projects[visible_start..visible_end]
            .iter()
            .enumerate()
            .map(|(relative_idx, project)| {
                let absolute_idx = visible_start + relative_idx;
                self.compute_display_item(project, absolute_idx, cols, matcher.as_ref())
            })
            .collect();

        let selected_display_index = self.selected_index.saturating_sub(visible_start);

        crate::ui::viewmodel::UIViewModel {
            display_items,
            selected_index: selected_display_index,
            header: self.compute_header(),
            footer: self.compute_footer(),
            empty_state: None,
            search_bar: self.compute_search_bar(),
        }
    }

    /// Computes a display item for a single project within the visible window.
    ///
    /// Handles name truncation, path formatting with prefix stripping, fuzzy match
    /// highlighting, and selection state marking.
    ///
    /// # Parameters
    ///
    /// * `project` - Project to render
    /// * `absolute_idx` - Index in `filtered_projects` (for selection comparison)
    /// * `cols` - Terminal width for responsive path truncation
    /// * `matcher` - Optional fuzzy matcher for highlight range computation
    ///
    /// # Returns
    ///
    /// A [`DisplayItem`](crate::ui::viewmodel::DisplayItem) with formatted fields
    /// and highlight ranges.
    fn compute_display_item(&self, project: &Project, absolute_idx: usize, cols: usize, matcher: Option<&SkimMatcherV2>) -> crate::ui::viewmodel::DisplayItem {
        const NAME_COLUMN_WIDTH: usize = 37;
        const SAFETY_MARGIN: usize = 2;

        let is_selected = absolute_idx == self.selected_index;
        let is_current_session = self.current_session.as_ref().is_some_and(|current| current == &project.name);
        let max_path_width = cols.saturating_sub(NAME_COLUMN_WIDTH + SAFETY_MARGIN);

        let name = if project.name.len() > 35 {
            format!("{}...", &project.name[..32])
        } else {
            project.name.clone()
        };

        let path = Self::format_display_path(&project.path, max_path_width);

        let highlight_ranges = matcher.map_or_else(Vec::new, |m| self.compute_highlight_ranges(&project.name, m));

        crate::ui::viewmodel::DisplayItem {
            name,
            path,
            is_selected,
            is_current_session,
            highlight_ranges,
        }
    }

    /// Computes character index ranges to highlight for fuzzy match visualization.
    ///
    /// Uses the Skim fuzzy matcher to find matching character positions, then
    /// coalesces consecutive indices into ranges for efficient highlighting.
    ///
    /// # Parameters
    ///
    /// * `text` - Text to search within (typically project name)
    /// * `matcher` - Fuzzy matcher instance
    ///
    /// # Returns
    ///
    /// A vector of `(start, end)` byte index ranges (exclusive end) representing
    /// contiguous highlighted segments.
    ///
    /// # Algorithm
    ///
    /// 1. Get fuzzy match indices from matcher
    /// 2. Iterate through indices, tracking consecutive runs
    /// 3. Emit a range when a gap is detected or at end
    /// 4. Return accumulated ranges
    fn compute_highlight_ranges(&self, text: &str, matcher: &SkimMatcherV2) -> Vec<(usize, usize)> {
        use fuzzy_matcher::FuzzyMatcher;

        if let Some((_score, indices)) = matcher.fuzzy_indices(text, &self.search_query) {
            let mut ranges = Vec::new();
            let mut start = None;
            let mut prev = None;

            for &idx in &indices {
                match (start, prev) {
                    (None, _) => {
                        start = Some(idx);
                        prev = Some(idx);
                    }
                    (Some(_), Some(p)) if idx == p + 1 => {
                        prev = Some(idx);
                    }
                    (Some(s), Some(p)) => {
                        ranges.push((s, p + 1));
                        start = Some(idx);
                        prev = Some(idx);
                    }
                    _ => {}
                }
            }

            if let (Some(s), Some(p)) = (start, prev) {
                ranges.push((s, p + 1));
            }

            ranges
        } else {
            vec![]
        }
    }

    /// Computes header information based on current view mode.
    ///
    /// Returns title text and count formatted for the UI header bar.
    ///
    /// # Returns
    ///
    /// A [`HeaderInfo`](crate::ui::viewmodel::HeaderInfo) with formatted title string.
    fn compute_header(&self) -> crate::ui::viewmodel::HeaderInfo {
        let (view_name, count) = match self.view_mode {
            ViewMode::Sessions => ("Active Sessions", self.filtered_projects.len()),
            ViewMode::ProjectsWithoutSessions => ("All Projects", self.filtered_projects.len()),
        };
        crate::ui::viewmodel::HeaderInfo {
            title: format!(" {view_name} ({count}) "),
        }
    }

    /// Computes footer keybindings text based on current input and view modes.
    ///
    /// Returns context-appropriate keybinding hints for the current mode combination.
    ///
    /// # Returns
    ///
    /// A [`FooterInfo`](crate::ui::viewmodel::FooterInfo) with keybinding text.
    fn compute_footer(&self) -> crate::ui::viewmodel::FooterInfo {
        use crate::app::modes::SearchFocus;

        let keybindings = match (self.input_mode, self.view_mode) {
            (InputMode::Search(SearchFocus::Typing), _) => {
                "ESC: exit search  Enter: select  Ctrl+n/p: navigate  Type to filter".to_string()
            }
            (InputMode::Search(SearchFocus::Navigating), _) => {
                "ESC: exit search  /: edit query  j/k or Ctrl+n/p: navigate  Enter: select".to_string()
            }
            (InputMode::Normal, ViewMode::Sessions) => {
                "j/k or Ctrl+n/p: navigate  /: search  n: new  K: kill  Enter: switch  q: quit".to_string()
            }
            (InputMode::Normal, ViewMode::ProjectsWithoutSessions) => {
                "j/k or Ctrl+n/p: navigate  /: search  s: sessions  Enter: create  q: quit".to_string()
            }
        };

        crate::ui::viewmodel::FooterInfo { keybindings }
    }

    /// Computes search bar state if in search mode.
    ///
    /// Returns `Some` with current query if search mode is active, `None` otherwise.
    ///
    /// # Returns
    ///
    /// An optional [`SearchBarInfo`](crate::ui::viewmodel::SearchBarInfo) with query text.
    fn compute_search_bar(&self) -> Option<crate::ui::viewmodel::SearchBarInfo> {
        if matches!(self.input_mode, InputMode::Search(_)) {
            Some(crate::ui::viewmodel::SearchBarInfo {
                query: self.search_query.clone(),
            })
        } else {
            None
        }
    }

    /// Calculates available rows for project list after subtracting UI chrome.
    ///
    /// Accounts for header (3 rows), footer (2 rows), borders (1 row), and
    /// search bar (3 rows if active).
    ///
    /// # Parameters
    ///
    /// * `total_rows` - Terminal height in character cells
    ///
    /// # Returns
    ///
    /// Number of rows available for project list display.
    const fn calculate_available_rows(&self, total_rows: usize) -> usize {
        match self.input_mode {
            InputMode::Normal => {
                total_rows.saturating_sub(6)
            }
            InputMode::Search(_) => {
                total_rows.saturating_sub(9)
            }
        }
    }

    /// Formats a project path for display, stripping prefix and truncating if needed.
    ///
    /// Removes the common path prefix (if set), then truncates from the start if
    /// the path exceeds the maximum width.
    ///
    /// # Parameters
    ///
    /// * `path` - Full project path
    /// * `max_width` - Maximum display width in characters
    ///
    /// # Returns
    ///
    /// A formatted path string, potentially with "..." prefix if truncated.
    fn format_display_path(path: &str, max_width: usize) -> String {
        if path.len() > max_width {
            let keep_chars = max_width.saturating_sub(3);
            format!("...{}", &path[path.len() - keep_chars..])
        } else {
            path.to_string()
        }
    }
}
