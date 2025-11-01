//! Storage backend abstraction.
//!
//! This module defines the [`Storage`] trait that abstracts over different persistence
//! backends. This allows seamless switching between storage implementations without
//! changing business logic.
//!
//! # Design Philosophy
//!
//! The trait is designed to be minimal and focused on the actual operations needed
//! by the application, not a generic ORM. Each method maps directly to a use case
//! in the worker thread.

use crate::domain::error::Result;
use crate::storage::models::{ProjectRecord, SessionRecord};

/// Abstraction over persistent storage backends.
///
/// Implementations must provide thread-safe access to project and session data
/// with support for frecency-based sorting and batch operations.
///
/// # Implementations
///
/// - [`JsonStorage`]: Uses JSON file with atomic writes (default)
///
/// # Examples
///
/// ```no_run
/// use zessionizer::storage::{Storage, JsonStorage};
/// use std::path::PathBuf;
///
/// let mut storage = JsonStorage::new(PathBuf::from("/tmp/projects.json"))?;
/// let projects = storage.get_all_projects()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub trait Storage: Send {
    /// Adds or updates a project in storage.
    ///
    /// If a project with the same path exists, updates it. Otherwise inserts new.
    /// Returns the ID or index of the stored project.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operation fails.
    fn add_project(&mut self, project: &ProjectRecord) -> Result<i64>;

    /// Adds or updates multiple projects in a single operation.
    ///
    /// More efficient than calling [`add_project`] in a loop. Returns the list
    /// of successfully stored projects (may differ from input due to deduplication).
    ///
    /// # Errors
    ///
    /// Returns an error if the batch operation fails. Some backends may perform
    /// partial writes before failing.
    fn add_projects_batch(&mut self, projects: &[ProjectRecord]) -> Result<Vec<ProjectRecord>>;

    /// Retrieves all projects from storage.
    ///
    /// Projects are returned unsorted. The caller is responsible for applying
    /// frecency sorting using [`crate::storage::frecency::calculate_score`].
    ///
    /// # Errors
    ///
    /// Returns an error if the read operation fails.
    fn get_all_projects(&self) -> Result<Vec<ProjectRecord>>;

    /// Updates the access timestamp and increments access count for a project.
    ///
    /// This is called when the user selects a project, maintaining frecency data.
    ///
    /// # Errors
    ///
    /// Returns an error if the project doesn't exist or the update fails.
    fn update_project_access(&mut self, path: &str, timestamp: i64) -> Result<()>;

    /// Retrieves a single project by its filesystem path.
    ///
    /// Returns `Ok(None)` if the project doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the read operation fails.
    fn get_project_by_path(&self, path: &str) -> Result<Option<ProjectRecord>>;

    /// Retrieves all session records.
    ///
    /// Sessions link Zellij session names to project paths, used for showing
    /// active session indicators in the UI.
    ///
    /// # Errors
    ///
    /// Returns an error if the read operation fails.
    fn get_all_sessions(&self) -> Result<Vec<SessionRecord>>;

    /// Synchronizes stored sessions with currently active Zellij sessions.
    ///
    /// Removes sessions that are no longer active and adds new ones. The exact
    /// implementation strategy (clear+insert vs diff) is backend-specific.
    ///
    /// # Errors
    ///
    /// Returns an error if the sync operation fails.
    fn sync_sessions(&mut self, active_session_names: &[String]) -> Result<()>;
}
