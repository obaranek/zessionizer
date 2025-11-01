//! Application layer coordinating state, events, and actions.
//!
//! This module defines the core application logic layer, sitting between the
//! plugin runtime (main.rs) and the domain/storage/worker layers. It implements
//! the event-driven architecture that powers the interactive UI.
//!
//! # Architecture
//!
//! The application layer follows a unidirectional data flow pattern:
//!
//! ```text
//! User Input → Events → Event Handler → State Mutations → Actions → Side Effects
//!                           ↑                                  ↓
//!                           └──────── Worker Responses ────────┘
//! ```
//!
//! # Modules
//!
//! - [`actions`]: Side effect commands emitted by the event handler
//! - [`handler`]: Event processing logic and state transition coordinator
//! - [`modes`]: Input and view mode state machine types
//! - [`state`]: Central application state container and view model computation
//!
//! # Example
//!
//! ```rust
//! use crate::app::{AppState, Event, handle_event};
//! use crate::ui::theme::Theme;
//!
//! let mut state = AppState::new(vec![], Theme::default());
//! let actions = handle_event(&mut state, &Event::KeyDown)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod actions;
pub mod handler;
pub mod modes;
pub mod state;

pub use actions::Action;
pub use handler::{handle_event, Event};
pub use modes::{InputMode, SearchFocus, ViewMode};
pub use state::AppState;
