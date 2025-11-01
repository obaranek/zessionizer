//! Composable UI component renderers.
//!
//! This module provides specialized rendering components for different UI
//! elements, following a component-based architecture. Each component is
//! responsible for rendering a specific part of the interface.
//!
//! # Components
//!
//! - [`header`]: Title bar with branding
//! - [`footer`]: Help text and keybinding hints
//! - [`search`]: Search input box (border, query text)
//! - [`table`]: Project/session list with columns (NAME, PATH)
//! - [`empty`]: Empty state message for no items
//!
//! # Layout Modes
//!
//! The module provides two high-level layout functions:
//!
//! - [`render_normal_mode`]: Header + Table + Footer
//! - [`render_search_mode`]: Header + `SearchBar` + Table + Footer
//!
//! # Example
//!
//! ```rust
//! use crate::ui::components::render_normal_mode;
//! use crate::ui::viewmodel::UIViewModel;
//! use crate::ui::Theme;
//!
//! let vm = UIViewModel { /* ... */ };
//! let theme = Theme::default();
//! render_normal_mode(&vm, &theme, 80, 24);
//! ```

mod header;
mod footer;
mod search;
mod table;
mod empty;

pub use empty::render_empty_state;

use crate::ui::theme::Theme;
use crate::ui::viewmodel::{UIViewModel, SearchBarInfo};
use crate::ui::helpers::position_cursor;

use header::render_header;
use footer::render_footer;
use search::render_search_bar;
use table::{render_table_headers, render_table_rows};

/// Renders a horizontal border line at the specified row.
///
/// Used to separate UI sections (header/table, table/footer).
///
/// # Parameters
///
/// * `row` - Row position to render the border (1-indexed)
/// * `color` - Hex color for the border
/// * `cols` - Terminal width in columns
///
/// # Returns
///
/// The next available row position (row + 1)
fn render_border(row: usize, color: &str, cols: usize) -> usize {
    position_cursor(row, 1);
    print!("{}", Theme::fg(color));
    print!("{}", "─".repeat(cols));
    print!("{}", Theme::reset());
    row + 1
}

/// Renders the normal mode layout (no search bar).
///
/// Layout structure:
/// ```text
/// [blank line]
/// [Header]
/// [Border]
/// [Table Headers]
/// [Table Rows]
/// [Blank padding to fill screen]
/// [Border]
/// [Footer]
/// ```
///
/// # Parameters
///
/// * `vm` - View model with display items and metadata
/// * `theme` - Active color theme
/// * `cols` - Terminal width in columns
/// * `rows` - Terminal height in rows
///
/// # Line Accounting
///
/// Reserves 6 lines for chrome (blank, header, 2 borders, header row, footer).
/// Fills remaining space with table rows and blank lines.
pub fn render_normal_mode(vm: &UIViewModel, theme: &Theme, cols: usize, rows: usize) {
    let mut current_row = 2; // Start at row 2 (skip blank line at row 1)

    current_row = render_header(current_row, &vm.header, theme, cols);
    current_row = render_border(current_row, &theme.colors.border, cols);
    current_row = render_table_headers(current_row, theme);
    let _current_row = render_table_rows(current_row, &vm.display_items, theme, cols);

    let footer_start = rows.saturating_sub(1);
    let border_row = footer_start.saturating_sub(1);

    render_border(border_row, &theme.colors.border, cols);
    render_footer(footer_start, &vm.footer, theme, cols);
}

/// Renders the search mode layout (with search bar).
///
/// Layout structure:
/// ```text
/// [blank line]
/// [Header]
/// [Border]
/// [Search Bar - 3 lines]
/// [Table Headers]
/// [Table Rows]
/// [Blank padding to fill screen]
/// [Border]
/// [Footer]
/// ```
///
/// # Parameters
///
/// * `vm` - View model with display items and metadata
/// * `search` - Search bar information (query text)
/// * `theme` - Active color theme
/// * `cols` - Terminal width in columns
/// * `rows` - Terminal height in rows
///
/// # Line Accounting
///
/// Reserves 9 lines for chrome (blank, header, 2 borders, search bar [3 lines],
/// header row, footer). Fills remaining space with table rows and blank lines.
pub fn render_search_mode(vm: &UIViewModel, search: &SearchBarInfo, theme: &Theme, cols: usize, rows: usize) {
    let mut current_row = 2; // Start at row 2 (skip blank line at row 1)

    current_row = render_header(current_row, &vm.header, theme, cols);
    current_row = render_border(current_row, &theme.colors.border, cols);
    current_row = render_search_bar(current_row, search, theme, cols);
    current_row = render_table_headers(current_row, theme);
    let _current_row = render_table_rows(current_row, &vm.display_items, theme, cols);

    let footer_start = rows.saturating_sub(1);
    let border_row = footer_start.saturating_sub(1);

    render_border(border_row, &theme.colors.border, cols);
    render_footer(footer_start, &vm.footer, theme, cols);
}
