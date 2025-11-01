//! Empty state component renderer.
//!
//! This module renders the empty state message displayed when no projects or
//! sessions are available.

use crate::ui::theme::Theme;
use crate::ui::viewmodel::EmptyState;
use crate::ui::helpers::position_cursor;

/// Renders the empty state message.
///
/// Displays a centered two-line message when no items are available. Typically
/// shown when:
/// - No projects have been scanned yet
/// - Search results are empty
/// - No sessions exist
///
/// # Parameters
///
/// * `empty` - Empty state information (message and subtitle)
/// * `theme` - Active color theme
/// * `cols` - Terminal width in columns
///
/// # Layout
///
/// ```text
/// [5 blank lines]
/// [left padding] MESSAGE [right padding]
/// [left padding] subtitle [right padding]
/// ```
///
/// Both lines are horizontally centered. The message uses the `empty_state_fg`
/// theme color, and the subtitle uses `text_dim` with dim styling. The message
/// is positioned starting at row 6, with the subtitle at row 7.
///
/// # Example
///
/// ```rust
/// use crate::ui::components::empty::render_empty_state;
/// use crate::ui::viewmodel::EmptyState;
/// use crate::ui::Theme;
///
/// let empty = EmptyState {
///     message: "No projects found".to_string(),
///     subtitle: "Press 'n' to scan for projects".to_string(),
/// };
/// let theme = Theme::default();
/// render_empty_state(&empty, &theme, 80);
/// ```
pub fn render_empty_state(empty: &EmptyState, theme: &Theme, cols: usize) {
    let msg_len = empty.message.len();
    let msg_padding = (cols.saturating_sub(msg_len)) / 2;

    position_cursor(6, 1);
    print!("{}", Theme::fg(&theme.colors.empty_state_fg));
    print!("{}", " ".repeat(msg_padding));
    print!("{}", empty.message);
    print!("{}", " ".repeat(cols.saturating_sub(msg_padding + msg_len)));
    print!("{}", Theme::reset());

    let sub_len = empty.subtitle.len();
    let sub_padding = (cols.saturating_sub(sub_len)) / 2;

    position_cursor(7, 1);
    print!("{}", Theme::dim());
    print!("{}", Theme::fg(&theme.colors.text_dim));
    print!("{}", " ".repeat(sub_padding));
    print!("{}", empty.subtitle);
    print!("{}", " ".repeat(cols.saturating_sub(sub_padding + sub_len)));
    print!("{}", Theme::reset());
}
