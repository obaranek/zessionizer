//! Search bar component renderer.
//!
//! This module renders the search input box with a bordered frame and query
//! text display.

use crate::ui::theme::Theme;
use crate::ui::viewmodel::SearchBarInfo;
use crate::ui::helpers::position_cursor;

/// Horizontal margin for the search box (spaces on left and right).
const SEARCH_BOX_MARGIN: usize = 5;

/// Renders the search input box at the specified row.
///
/// Displays a 3-line bordered box containing the search query text. The box
/// is horizontally centered with margins on both sides.
///
/// # Parameters
///
/// * `row` - Starting row position for the search box (1-indexed)
/// * `search` - Search bar information (query text)
/// * `theme` - Active color theme
/// * `cols` - Terminal width in columns
///
/// # Returns
///
/// The next available row position (row + 3, since search box uses 3 lines)
///
/// # Layout
///
/// ```text
/// [margin] ┌─────────────┐ [margin]
/// [margin] │ Search: ... │ [margin]
/// [margin] └─────────────┘ [margin]
/// ```
///
/// The box width is calculated as `cols - (2 * SEARCH_BOX_MARGIN)`. The inner
/// content width is `box_width - 2` (accounting for left and right borders).
///
/// # Rendering Details
///
/// - Borders use theme `search_bar_border` color
/// - Query text uses theme `text_normal` color
/// - Query is displayed as " Search: {query}"
/// - Right padding fills remaining space to box edge
///
/// # Example
///
/// ```rust
/// use crate::ui::components::search::render_search_bar;
/// use crate::ui::viewmodel::SearchBarInfo;
/// use crate::ui::Theme;
///
/// let search = SearchBarInfo { query: "proj".to_string() };
/// let theme = Theme::default();
/// let next_row = render_search_bar(1, &search, &theme, 80);
/// ```
pub fn render_search_bar(row: usize, search: &SearchBarInfo, theme: &Theme, cols: usize) -> usize {
    let box_width = cols.saturating_sub(SEARCH_BOX_MARGIN * 2);
    let inner_width = box_width.saturating_sub(2);

    position_cursor(row, 1);
    print!("{}", " ".repeat(SEARCH_BOX_MARGIN));
    print!("{}", Theme::fg(&theme.colors.search_bar_border));
    print!("┌{}┐", "─".repeat(inner_width));
    print!("{}", Theme::reset());

    let search_text = format!(" Search: {}", search.query);
    let padding = inner_width.saturating_sub(search_text.len());

    position_cursor(row + 1, 1);
    print!("{}", " ".repeat(SEARCH_BOX_MARGIN));
    print!("{}", Theme::fg(&theme.colors.search_bar_border));
    print!("│");
    print!("{}", Theme::fg(&theme.colors.text_normal));
    print!("{search_text}");
    print!("{}", " ".repeat(padding));
    print!("{}", Theme::fg(&theme.colors.search_bar_border));
    print!("│");
    print!("{}", Theme::reset());

    position_cursor(row + 2, 1);
    print!("{}", " ".repeat(SEARCH_BOX_MARGIN));
    print!("{}", Theme::fg(&theme.colors.search_bar_border));
    print!("└{}┘", "─".repeat(inner_width));
    print!("{}", Theme::reset());

    row + 3
}
