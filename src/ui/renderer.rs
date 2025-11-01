//! Top-level rendering coordinator.
//!
//! This module provides the main rendering entry point, coordinating view model
//! computation and delegation to UI components. It handles mode switching
//! (normal, search, empty state) and ensures proper layout filling.
//!
//! # Architecture
//!
//! The renderer follows a two-step process:
//!
//! 1. **View Model Computation**: Transform `AppState` into `UIViewModel`
//! 2. **Component Rendering**: Delegate to specialized component renderers
//!
//! # Example
//!
//! ```rust
//! use crate::ui::render;
//! use crate::app::AppState;
//! use crate::ui::Theme;
//!
//! let state = AppState::new(vec![], Theme::default());
//! render(&state, 24, 80); // Render to stdout
//! ```

use crate::app::AppState;
use crate::ui::components;
use crate::ui::viewmodel::UIViewModel;
use crate::ui::theme::Theme;

/// Renders the plugin UI to stdout.
///
/// Computes the view model from application state and delegates to the
/// appropriate rendering mode (normal, search, or empty state).
///
/// # Parameters
///
/// * `state` - Current application state
/// * `rows` - Terminal height in rows
/// * `cols` - Terminal width in columns
///
/// # Output
///
/// Prints ANSI-styled output to stdout using `print!` and `println!` macros.
/// Does not clear the screen or manage cursor position.
///
/// # Example
///
/// ```rust
/// use crate::ui::render;
/// use crate::app::AppState;
///
/// let state = AppState::new(vec![], Default::default());
/// render(&state, 24, 80);
/// ```
pub fn render(state: &AppState, rows: usize, cols: usize) {
    let viewmodel = state.compute_viewmodel(rows, cols);

    render_viewmodel(&viewmodel, &state.theme, rows, cols);
}

/// Renders a view model with mode-specific layout.
///
/// Chooses rendering strategy based on view model state:
/// - Empty state: Centered message display
/// - Search mode: Header, search bar, table, footer
/// - Normal mode: Header, table, footer
///
/// # Parameters
///
/// * `vm` - Pre-computed view model
/// * `theme` - Active color theme
/// * `rows` - Terminal height in rows
/// * `cols` - Terminal width in columns
fn render_viewmodel(vm: &UIViewModel, theme: &Theme, rows: usize, cols: usize) {
    if let Some(empty) = &vm.empty_state {
        components::render_empty_state(empty, theme, cols);
        return;
    }

    if let Some(search) = &vm.search_bar {
        components::render_search_mode(vm, search, theme, cols, rows);
    } else {
        components::render_normal_mode(vm, theme, cols, rows);
    }
}
