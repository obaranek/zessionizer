//! Theme management and ANSI escape sequence generation.
//!
//! This module defines the color scheme system for the plugin, supporting both
//! built-in themes (Catppuccin variants) and custom themes loaded from TOML files.
//! It provides utilities for converting hex colors to ANSI escape sequences.
//!
//! # Built-in Themes
//!
//! - `catppuccin-mocha`: Dark theme with warm tones (default)
//! - `catppuccin-latte`: Light theme with soft pastels
//! - `catppuccin-frappe`: Cool dark theme
//! - `catppuccin-macchiato`: Warm dark theme
//!
//! # TOML Format
//!
//! ```toml
//! name = "my-theme"
//!
//! [colors]
//! header_fg = "#cdd6f4"
//! selection_fg = "#1e1e2e"
//! selection_bg = "#f5c2e7"
//! text_normal = "#cdd6f4"
//! text_dim = "#6c7086"
//! border = "#45475a"
//! search_bar_border = "#f5c2e7"
//! match_highlight_fg = "#1e1e2e"
//! match_highlight_bg = "#f9e2af"
//! empty_state_fg = "#89b4fa"
//! active_session_fg = "#f9e2af"
//! ```
//!
//! # Example
//!
//! ```rust
//! use crate::ui::theme::Theme;
//!
//! let theme = Theme::from_name("catppuccin-mocha").unwrap();
//! println!("{}", Theme::fg(&theme.colors.header_fg));
//! println!("{}Bold Text{}", Theme::bold(), Theme::reset());
//! ```

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Color scheme configuration for UI rendering.
///
/// Contains theme metadata and color definitions. Can be loaded from built-in
/// themes or custom TOML files.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Theme {
    /// Human-readable theme name.
    pub name: String,
    /// Color palette for all UI elements.
    pub colors: ThemeColors,
}

/// Color definitions for all UI elements.
///
/// All colors are specified as hex strings (e.g., "#cdd6f4"). Optional fields
/// default to `None`, allowing themes to opt out of certain styling.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeColors {
    /// Header text color.
    pub header_fg: String,
    /// Optional header background color.
    #[serde(default)]
    pub header_bg: Option<String>,

    /// Selected row foreground color.
    pub selection_fg: String,
    /// Selected row background color.
    pub selection_bg: String,

    /// Normal text color.
    pub text_normal: String,
    /// Dimmed text color (footer, secondary info).
    pub text_dim: String,

    /// Border and separator line color.
    pub border: String,

    /// Search bar border color.
    pub search_bar_border: String,
    /// Fuzzy match highlight foreground.
    pub match_highlight_fg: String,
    /// Fuzzy match highlight background.
    pub match_highlight_bg: String,

    /// Empty state message color.
    pub empty_state_fg: String,

    /// Active session indicator color.
    pub active_session_fg: String,
}

impl Theme {
    /// Loads a built-in theme by name.
    ///
    /// Supported names: `catppuccin-mocha`, `catppuccin-latte`,
    /// `catppuccin-frappe`, `catppuccin-macchiato`.
    ///
    /// # Returns
    ///
    /// - `Some(Theme)` if the theme name is recognized
    /// - `None` if the theme name is unknown
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::ui::theme::Theme;
    ///
    /// let theme = Theme::from_name("catppuccin-mocha").unwrap();
    /// assert_eq!(theme.name, "catppuccin-mocha");
    /// ```
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let toml_str = match name {
            "catppuccin-mocha" => include_str!("../../themes/catppuccin-mocha.toml"),
            "catppuccin-latte" => include_str!("../../themes/catppuccin-latte.toml"),
            "catppuccin-frappe" => include_str!("../../themes/catppuccin-frappe.toml"),
            "catppuccin-macchiato" => include_str!("../../themes/catppuccin-macchiato.toml"),
            _ => return None,
        };

        toml::from_str(toml_str).ok()
    }

    /// Loads a theme from a TOML file.
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the TOML file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read (file not found, permission denied, etc.)
    /// - The TOML content cannot be parsed (invalid syntax, missing fields, type mismatches)
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::ui::theme::Theme;
    ///
    /// let theme = Theme::from_file("/path/to/theme.toml")?;
    /// # Ok::<(), String>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read theme file: {e}"))?;

        toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse theme TOML: {e}"))
    }

    /// Converts a hex color to RGB tuple.
    ///
    /// Strips `#` prefix if present, validates length, and parses hex digits.
    /// Returns `(255, 255, 255)` (white) on parse errors.
    ///
    /// # Parameters
    ///
    /// * `hex` - Hex color string (e.g., "#cdd6f4" or "cdd6f4")
    ///
    /// # Returns
    ///
    /// An `(r, g, b)` tuple with values 0-255.
    fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
        let hex = hex.trim_start_matches('#').trim();

        if hex.len() != 6 {
            return (255, 255, 255);
        }

        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);

        (r, g, b)
    }

    /// Generates an ANSI 24-bit foreground color escape sequence.
    ///
    /// Converts a hex color to RGB and formats as `\x1b[38;2;r;g;bm`.
    ///
    /// # Parameters
    ///
    /// * `hex` - Hex color string (e.g., "#cdd6f4")
    ///
    /// # Returns
    ///
    /// An ANSI escape sequence string for foreground color.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::ui::theme::Theme;
    ///
    /// let fg = Theme::fg("#cdd6f4");
    /// print!("{}Colored text{}", fg, Theme::reset());
    /// ```
    #[must_use]
    pub fn fg(hex: &str) -> String {
        let (r, g, b) = Self::hex_to_rgb(hex);
        format!("\u{001b}[38;2;{r};{g};{b}m")
    }

    /// Generates an ANSI 24-bit background color escape sequence.
    ///
    /// Converts a hex color to RGB and formats as `\x1b[48;2;r;g;bm`.
    ///
    /// # Parameters
    ///
    /// * `hex` - Hex color string (e.g., "#f5c2e7")
    ///
    /// # Returns
    ///
    /// An ANSI escape sequence string for background color.
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::ui::theme::Theme;
    ///
    /// let bg = Theme::bg("#f5c2e7");
    /// print!("{}Highlighted{}", bg, Theme::reset());
    /// ```
    #[must_use]
    pub fn bg(hex: &str) -> String {
        let (r, g, b) = Self::hex_to_rgb(hex);
        format!("\u{001b}[48;2;{r};{g};{b}m")
    }

    /// Returns the ANSI bold escape sequence (`\x1b[1m`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::ui::theme::Theme;
    ///
    /// print!("{}Bold text{}", Theme::bold(), Theme::reset());
    /// ```
    #[must_use]
    pub const fn bold() -> &'static str {
        "\u{001b}[1m"
    }

    /// Returns the ANSI dim escape sequence (`\x1b[2m`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::ui::theme::Theme;
    ///
    /// print!("{}Dimmed text{}", Theme::dim(), Theme::reset());
    /// ```
    #[must_use]
    pub const fn dim() -> &'static str {
        "\u{001b}[2m"
    }

    /// Returns the ANSI reset escape sequence (`\x1b[0m`).
    ///
    /// Clears all styling (colors, bold, dim, etc.).
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::ui::theme::Theme;
    ///
    /// print!("{}Styled{} Normal", Theme::bold(), Theme::reset());
    /// ```
    #[must_use]
    pub const fn reset() -> &'static str {
        "\u{001b}[0m"
    }
}

impl Default for Theme {
    /// Returns the default theme (Catppuccin Mocha).
    ///
    /// # Panics
    ///
    /// Panics if the built-in theme fails to parse (should never occur).
    ///
    /// # Example
    ///
    /// ```rust
    /// use zessionizer::ui::Theme;
    ///
    /// let theme = Theme::default();
    /// assert_eq!(theme.name, "catppuccin-mocha");
    /// ```
    fn default() -> Self {
        Self::from_name("catppuccin-mocha")
            .expect("Built-in catppuccin-mocha theme should always parse")
    }
}
