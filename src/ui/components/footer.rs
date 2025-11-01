//! Footer component renderer.
//!
//! This module renders the footer help bar with centered keybinding hints.

use crate::ui::theme::Theme;
use crate::ui::viewmodel::FooterInfo;
use crate::ui::helpers::position_cursor;

/// Renders the footer help bar at the specified row.
///
/// Displays keybinding hints centered horizontally with dimmed styling. Pads
/// the line to fill the entire terminal width.
///
/// # Parameters
///
/// * `row` - Row position to render the footer (1-indexed)
/// * `footer` - Footer information (keybinding text)
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
/// [left padding] keybindings [right padding]
/// ```
///
/// Padding is split evenly on both sides to center the text. If the terminal
/// width cannot evenly divide, left padding is slightly larger.
///
/// # Truncation
///
/// If the help text exceeds terminal width, it is truncated to fit. This
/// prevents layout corruption on narrow terminals.
///
/// # Example
///
/// ```rust
/// use crate::ui::components::footer::render_footer;
/// use crate::ui::viewmodel::FooterInfo;
/// use crate::ui::Theme;
///
/// let footer = FooterInfo {
///     keybindings: "q: quit | /: search | n: projects".to_string()
/// };
/// let theme = Theme::default();
/// let next_row = render_footer(1, &footer, &theme, 80);
/// ```
pub fn render_footer(row: usize, footer: &FooterInfo, theme: &Theme, cols: usize) -> usize {
    let help_text = &footer.keybindings;

    let text_len = help_text.len().min(cols);
    let padding = (cols.saturating_sub(text_len)) / 2;

    position_cursor(row, 1);
    print!("{}", Theme::fg(&theme.colors.text_dim));
    print!("{}", " ".repeat(padding));
    print!("{help_text}");
    print!("{}", " ".repeat(cols.saturating_sub(padding + text_len)));
    print!("{}", Theme::reset());
    row + 1
}
