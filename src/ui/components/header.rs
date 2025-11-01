//! Header component renderer.
//!
//! This module renders the plugin title bar with centered text, theme-aware
//! colors, and optional background styling.

use crate::ui::theme::Theme;
use crate::ui::viewmodel::HeaderInfo;
use crate::ui::helpers::position_cursor;

/// Renders the header title bar at the specified row.
///
/// Displays the title centered horizontally with bold styling and theme colors.
/// Pads the line to fill the entire terminal width.
///
/// # Parameters
///
/// * `row` - Row position to render the header (1-indexed)
/// * `header` - Header information (title text)
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
/// [left padding] TITLE [right padding]
/// ```
///
/// Padding is split evenly on both sides to center the title. If the terminal
/// width cannot evenly divide, left padding is slightly larger.
///
/// # Example
///
/// ```rust
/// use crate::ui::components::header::render_header;
/// use crate::ui::viewmodel::HeaderInfo;
/// use crate::ui::Theme;
///
/// let header = HeaderInfo { title: "Zessionizer".to_string() };
/// let theme = Theme::default();
/// let next_row = render_header(1, &header, &theme, 80);
/// // Output: "                                  Zessionizer                                  "
/// ```
pub fn render_header(row: usize, header: &HeaderInfo, theme: &Theme, cols: usize) -> usize {
    let title_len = header.title.len();
    let padding = (cols.saturating_sub(title_len)) / 2;

    position_cursor(row, 1);
    print!("{}", Theme::bold());
    print!("{}", Theme::fg(&theme.colors.header_fg));
    if let Some(bg) = &theme.colors.header_bg {
        print!("{}", Theme::bg(bg));
    }

    print!("{}", " ".repeat(padding));
    print!("{}", header.title);
    print!("{}", " ".repeat(cols.saturating_sub(padding + title_len)));

    print!("{}", Theme::reset());
    row + 1
}
