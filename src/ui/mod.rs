//! User interface rendering layer with component-based architecture.
//!
//! This module orchestrates the terminal-based UI, transforming view models into
//! ANSI-styled output through composable rendering components. It provides theme
//! support, responsive layout, and fuzzy match highlighting.
//!
//! # Architecture
//!
//! The UI layer follows a declarative rendering model:
//!
//! ```text
//! AppState → compute_viewmodel → UIViewModel → render → ANSI Output
//! ```
//!
//! # Modules
//!
//! - [`viewmodel`]: View model types representing renderable UI state
//! - [`renderer`]: Top-level rendering coordinator
//! - [`components`]: Composable UI component renderers
//! - [`helpers`]: Shared rendering utilities (highlighting, formatting)
//! - [`theme`]: Color scheme definitions and ANSI escape sequence generation
//!
//! # Example
//!
//! ```rust
//! use crate::app::AppState;
//! use crate::ui::{render, Theme};
//!
//! let state = AppState::new(vec![], Theme::default());
//! render(&state, 24, 80); // Renders to stdout
//! ```

pub mod viewmodel;
pub mod renderer;
pub mod components;
pub mod helpers;
pub mod theme;

pub use viewmodel::{
    UIViewModel, DisplayItem, HeaderInfo, FooterInfo, EmptyState, SearchBarInfo,
};
pub use renderer::render;
pub use theme::Theme;
