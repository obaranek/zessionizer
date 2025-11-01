//! Table component renderer.
//!
//! This module renders the project/session list as a two-column table with
//! NAME and PATH columns. It supports selection highlighting and fuzzy match
//! highlighting.

use crate::ui::theme::Theme;
use crate::ui::viewmodel::DisplayItem;
use crate::ui::helpers::{self, position_cursor};

/// Renders the table column headers at the specified row.
///
/// Displays "NAME" and "PATH" column headers with bold styling and theme colors.
/// Uses fixed column width (37 characters for NAME).
///
/// # Parameters
///
/// * `row` - Row position to render the headers (1-indexed)
/// * `theme` - Active color theme
///
/// # Returns
///
/// The next available row position (row + 1)
///
/// # Example
///
/// ```rust
/// use crate::ui::components::table::render_table_headers;
/// use crate::ui::Theme;
///
/// let theme = Theme::default();
/// let next_row = render_table_headers(1, &theme);
/// // Output: "NAME                                 PATH"
/// ```
pub fn render_table_headers(row: usize, theme: &Theme) -> usize {
    position_cursor(row, 1);
    print!("{}", Theme::bold());
    print!("{}", Theme::fg(&theme.colors.header_fg));
    print!("{:<37} {:<}", "NAME", "PATH");
    print!("{}", Theme::reset());
    row + 1
}

/// Renders all table rows starting at the specified row.
///
/// Iterates through display items and renders each as a table row with proper
/// selection and highlight styling.
///
/// # Parameters
///
/// * `row` - Starting row position for the table (1-indexed)
/// * `items` - List of display items to render
/// * `theme` - Active color theme
/// * `cols` - Terminal width in columns (for padding)
///
/// # Returns
///
/// The next available row position (row + number of items)
pub fn render_table_rows(row: usize, items: &[DisplayItem], theme: &Theme, cols: usize) -> usize {
    let mut current_row = row;
    for item in items {
        current_row = render_table_row(current_row, item, theme, cols);
    }
    current_row
}

/// Renders a single table row at the specified row position.
///
/// Displays one project/session with:
/// - NAME column (37 chars fixed width, left-aligned)
/// - PATH column (remaining width, left-aligned)
/// - Selection highlighting (full row background)
/// - Fuzzy match highlighting (character ranges)
///
/// # Parameters
///
/// * `row` - Row position to render this item (1-indexed)
/// * `item` - Display item to render
/// * `theme` - Active color theme
/// * `cols` - Terminal width in columns
///
/// # Returns
///
/// The next available row position (row + 1)
///
/// # Layout
///
/// ```text
/// NAME (up to 35 chars) [2 spaces] PATH (variable) [padding to fill line]
/// ```
///
/// # Styling Precedence
///
/// 1. Selection background (if `is_selected`)
/// 2. Fuzzy match highlights (unless selected)
/// 3. Normal text color
///
/// The row is padded to fill the entire terminal width to ensure consistent
/// selection background rendering.
fn render_table_row(row: usize, item: &DisplayItem, theme: &Theme, cols: usize) -> usize {
    position_cursor(row, 1);

    if item.is_selected {
        print!("{}", Theme::fg(&theme.colors.selection_fg));
        print!("{}", Theme::bg(&theme.colors.selection_bg));
    } else {
        print!("{}", Theme::fg(&theme.colors.text_normal));
    }

    if item.is_current_session {
        print!("{}", Theme::fg(&theme.colors.active_session_fg));
        print!("* ");
        if item.is_selected {
            print!("{}", Theme::fg(&theme.colors.selection_fg));
        } else {
            print!("{}", Theme::fg(&theme.colors.text_normal));
        }
    }

    if item.highlight_ranges.is_empty() {
        print!("{}", item.name);
    } else {
        helpers::render_highlighted_text(
            &item.name,
            &item.highlight_ranges,
            theme,
            item.is_selected,
        );
    }

    let indicator_len = if item.is_current_session { 2 } else { 0 };
    let name_visual_len = item.name.len().min(35) + indicator_len;
    print!("{}", " ".repeat(37_usize.saturating_sub(name_visual_len)));

    print!("{}", item.path);
    let path_len = item.path.len();

    let line_len = 37 + path_len;
    let padding = cols.saturating_sub(line_len);
    print!("{}", " ".repeat(padding));

    print!("{}", Theme::reset());
    row + 1
}
