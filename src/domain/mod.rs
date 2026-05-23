//! Domain layer for the Zessionizer plugin.
//!
//! This module contains the core domain types and business logic for the plugin,
//! independent of Zellij-specific APIs or infrastructure concerns. It follows
//! domain-driven design principles by keeping business rules isolated from external
//! dependencies.
//!
//! # Organization
//!
//! - [`error`]: Error types and result aliases
//! - [`project`]: Project domain model and operations
//!
//! # Examples
//!
//! ```
//! use zessionizer::domain::{Project, Result};
//!
//! fn create_project() -> Result<Project> {
//!     Ok(Project::new(
//!         "/home/user/code/myproject".to_string(),
//!         "myproject".to_string()
//!     ))
//! }
//! ```

pub mod error;
pub mod project;

pub use error::{Result, ZessionizerError};
pub use project::Project;
