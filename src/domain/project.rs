//! Project domain model and operations.
//!
//! This module defines the core `Project` type representing a Git repository directory
//! that can be opened in Zellij sessions. Projects track access patterns for frecency-based
//! sorting (frequency + recency) and provide user-friendly time formatting.

use serde::{Deserialize, Serialize};

/// Number of seconds in one minute.
const SECONDS_PER_MINUTE: i64 = 60;

/// Number of seconds in one hour.
const SECONDS_PER_HOUR: i64 = 3600;

/// Number of seconds in one day.
const SECONDS_PER_DAY: i64 = 86400;

/// Represents a project that can be opened in Zellij.
///
/// A project is a Git repository directory that can be opened in Zellij sessions.
/// Projects track access patterns for frecency-based sorting (frequency + recency)
/// to prioritize frequently and recently used repositories.
///
/// # Fields
///
/// - `id`: Storage identifier, `None` for new projects not yet persisted
/// - `path`: Absolute filesystem path to the project directory
/// - `name`: Display name derived from the directory name
/// - `last_accessed`: Unix timestamp of most recent access
/// - `created_at`: Unix timestamp when the project was first added
/// - `layout`: Optional layout to use when creating a session for this project
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: Option<i64>,
    pub path: String,
    pub name: String,
    pub last_accessed: i64,
    pub created_at: i64,
    pub layout: Option<String>,
}

impl Project {
    /// Creates a new project with the given path and name.
    ///
    /// Both `last_accessed` and `created_at` timestamps are set to the current time.
    /// The `id` field is set to `None` until the project is persisted to storage.
    /// The `layout` field is set to `None` initially.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::domain::Project;
    ///
    /// let project = Project::new(
    ///     "/home/user/code/myproject".to_string(),
    ///     "myproject".to_string()
    /// );
    /// assert_eq!(project.path, "/home/user/code/myproject");
    /// assert_eq!(project.name, "myproject");
    /// assert!(project.id.is_none());
    /// assert!(project.layout.is_none());
    /// ```
    #[must_use]
    pub fn new(path: String, name: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: None,
            path,
            name,
            last_accessed: now,
            created_at: now,
            layout: None,
        }
    }

    /// Returns a human-readable string describing how long ago the project was accessed.
    ///
    /// The format varies based on the time elapsed:
    /// - Less than 1 minute: "just now"
    /// - Less than 1 hour: "Xm ago" (e.g., "5m ago")
    /// - Less than 1 day: "Xh ago" (e.g., "3h ago")
    /// - 1 day or more: "Xd ago" (e.g., "7d ago")
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::domain::Project;
    ///
    /// let mut project = Project::new(
    ///     "/home/user/code/myproject".to_string(),
    ///     "myproject".to_string()
    /// );
    ///
    /// // Project just created
    /// assert_eq!(project.time_ago(), "just now");
    ///
    /// // Simulate project accessed 5 minutes ago
    /// project.last_accessed = chrono::Utc::now().timestamp() - 300;
    /// assert_eq!(project.time_ago(), "5m ago");
    /// ```
    #[must_use]
    pub fn time_ago(&self) -> String {
        let now = chrono::Utc::now().timestamp();
        let diff = now - self.last_accessed;

        if diff < SECONDS_PER_MINUTE {
            "just now".to_string()
        } else if diff < SECONDS_PER_HOUR {
            let mins = diff / SECONDS_PER_MINUTE;
            format!("{mins}m ago")
        } else if diff < SECONDS_PER_DAY {
            let hours = diff / SECONDS_PER_HOUR;
            format!("{hours}h ago")
        } else {
            let days = diff / SECONDS_PER_DAY;
            format!("{days}d ago")
        }
    }
}
