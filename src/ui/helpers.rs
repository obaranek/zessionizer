//! Shared rendering utilities and helpers.
//!
//! This module provides low-level rendering utilities used across multiple UI
//! components. It handles complex text rendering tasks like fuzzy match
//! highlighting with proper ANSI escape sequence management.
//!
//! # Features
//!
//! - **Fuzzy Match Highlighting**: Renders text with highlighted character ranges
//! - **Selection Awareness**: Adjusts highlighting based on selection state
//! - **UTF-8 Safe**: Operates on character indices, not byte indices
//!
//! # Example
//!
//! ```rust
//! use crate::ui::helpers::render_highlighted_text;
//! use crate::ui::Theme;
//!
//! let theme = Theme::default();
//! let text = "my-project";
//! let ranges = vec![(0, 2), (3, 4)]; // Highlight "my" and "p"
//!
//! render_highlighted_text(text, &ranges, &theme, false);
//! // Outputs: "\x1b[38;2;...my\x1b[0m-\x1b[38;2;...p\x1b[0mroject"
//! ```

use crate::ui::theme::Theme;

/// Positions the cursor at a specific row and column.
///
/// Uses ANSI escape sequence `\u{1b}[{row};{col}H` to move the cursor.
/// Coordinates are 1-indexed (row 1 = first row, col 1 = first column).
///
/// # Parameters
///
/// * `row` - Target row (1-indexed)
/// * `col` - Target column (1-indexed, typically 1 for start of line)
///
/// # Example
///
/// ```rust
/// use crate::ui::helpers::position_cursor;
///
/// position_cursor(5, 1); // Move to start of row 5
/// print!("Content at row 5");
/// ```
pub fn position_cursor(row: usize, col: usize) {
    print!("\u{1b}[{row};{col}H");
}

/// Renders text with highlighted character ranges for fuzzy matches.
///
/// Splits the text into highlighted and normal sections based on the provided
/// character ranges. Highlighted sections use match highlight colors unless the
/// item is selected, in which case selection colors take precedence.
///
/// # Parameters
///
/// * `text` - The text to render
/// * `ranges` - Character index ranges to highlight `(start, end)` (inclusive start, exclusive end)
/// * `theme` - Active color theme for highlight colors
/// * `is_selected` - Whether the item is currently selected (disables match highlighting)
///
/// # Character Indices
///
/// Ranges use UTF-8 character indices (not byte indices). The function converts
/// the text to a character vector for proper indexing.
///
/// # Selection Behavior
///
/// When `is_selected` is `true`, match highlighting is disabled to avoid
/// conflicting with selection background colors.
///
/// # Output
///
/// Prints to stdout using ANSI escape sequences:
/// - Normal sections: Theme default text color
/// - Highlighted sections: `match_highlight_fg` + `match_highlight_bg`
/// - Selection restoration: Re-applies selection colors after highlights
///
/// # Example
///
/// ```rust
/// use crate::ui::helpers::render_highlighted_text;
/// use crate::ui::Theme;
///
/// let theme = Theme::default();
/// render_highlighted_text("my-project", &[(0, 2)], &theme, false);
/// // Prints "my-project" with "my" highlighted
/// ```
pub fn render_highlighted_text(
    text: &str,
    ranges: &[(usize, usize)],
    theme: &Theme,
    is_selected: bool,
) {
    if ranges.is_empty() || is_selected {
        print!("{text}");
        return;
    }

    let chars: Vec<char> = text.chars().collect();
    let mut current_pos = 0;

    for &(start, end) in ranges {
        if start > current_pos {
            let normal_section: String = chars[current_pos..start].iter().collect();
            print!("{normal_section}");
        }

        print!("{}", Theme::fg(&theme.colors.match_highlight_fg));
        print!("{}", Theme::bg(&theme.colors.match_highlight_bg));
        let highlighted_section: String = chars[start..end.min(chars.len())].iter().collect();
        print!("{highlighted_section}");
        print!("{}", Theme::reset());

        if is_selected {
            print!("{}", Theme::fg(&theme.colors.selection_fg));
            print!("{}", Theme::bg(&theme.colors.selection_bg));
        }

        current_pos = end;
    }

    if current_pos < chars.len() {
        let remaining: String = chars[current_pos..].iter().collect();
        print!("{remaining}");
    }
}
