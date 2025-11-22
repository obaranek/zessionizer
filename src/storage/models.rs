//! Storage record models for persistence layer.
//!
//! This module defines the raw storage record types used for persistence operations.
//! These types are separate from domain models to maintain a clear boundary between
//! storage representation and business logic.

use serde::{Deserialize, Serialize};

/// Represents a project record in storage.
///
/// This is the storage-layer representation of a project, containing all fields
/// necessary for persistence and frecency-based sorting. Unlike the domain `Project`,
/// this record includes storage-specific fields like `access_count`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRecord {
    /// Absolute filesystem path to the project directory.
    pub path: String,

    /// Display name derived from the directory name.
    pub name: String,

    /// Unix timestamp of most recent access, `None` if never accessed.
    pub last_accessed: Option<i64>,

    /// Number of times the project has been accessed.
    pub access_count: i32,

    /// Unix timestamp when the project was first added to storage.
    pub created_at: i64,

    /// Optional layout to use when creating a session for this project
    pub layout: Option<String>,
}

impl ProjectRecord {
    /// Creates a new project record with default values.
    ///
    /// Sets `access_count` to 1, `last_accessed` to `None`, `layout` to `None`, and `created_at` to the current time.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::storage::ProjectRecord;
    ///
    /// let record = ProjectRecord::new(
    ///     "/home/user/code/myproject",
    ///     "myproject"
    /// );
    /// assert_eq!(record.access_count, 1);
    /// assert!(record.last_accessed.is_none());
    /// assert!(record.layout.is_none());
    /// ```
    pub fn new(path: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            last_accessed: None,
            access_count: 1,
            created_at: chrono::Utc::now().timestamp(),
            layout: None,
        }
    }
}

/// Represents a session record linking Zellij sessions to projects.
///
/// Sessions track which Zellij session names are associated with which projects,
/// enabling the plugin to show active session indicators in the project list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Zellij session name.
    pub name: String,

    /// Absolute filesystem path to the associated project.
    pub project_path: String,
}

impl SessionRecord {
    /// Creates a new session record.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::storage::SessionRecord;
    ///
    /// let session = SessionRecord::new("my-session", "/home/user/code/myproject");
    /// assert_eq!(session.name, "my-session");
    /// ```
    pub fn new(name: impl Into<String>, project_path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            project_path: project_path.into(),
        }
    }
}
