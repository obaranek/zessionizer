//! JSON file-based storage backend.
//!
//! This module provides a simple, human-readable storage implementation using
//! JSON serialization. It uses atomic file writes (write-to-temp + rename) to
//! prevent corruption on crashes.
//!
//! # Performance Characteristics
//!
//! - **Read**: O(1) - loads entire file into memory once
//! - **Write**: O(n) - serializes and writes entire dataset
//! - **Best for**: < 1000 projects, infrequent writes
//! - **Binary size**: ~1.8MB (no external dependencies)

use crate::domain::error::{Result, ZessionizerError};
use crate::storage::backend::Storage;
use crate::storage::models::{ProjectRecord, SessionRecord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// JSON storage container format.
///
/// This is the top-level structure serialized to disk. Wraps projects and
/// sessions in a single object for better JSON structure and future extensibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StorageData {
    /// Version of the storage format for future migrations.
    version: u32,

    /// All stored projects, indexed by path for O(1) lookups.
    #[serde(default)]
    projects: HashMap<String, ProjectRecord>,

    /// Active sessions linking session names to project paths.
    #[serde(default)]
    sessions: Vec<SessionRecord>,
}

impl Default for StorageData {
    fn default() -> Self {
        Self {
            version: 1,
            projects: HashMap::new(),
            sessions: Vec::new(),
        }
    }
}

/// JSON file storage backend.
///
/// Stores projects and sessions in a human-readable JSON file with atomic writes.
/// The entire dataset is kept in memory and persisted on modifications.
///
/// # Thread Safety
///
/// This type is `Send` but not `Sync`. It's designed to be used from a single
/// worker thread, matching the Zellij plugin architecture.
///
/// # File Format
///
/// ```json
/// {
///   "version": 1,
///   "projects": {
///     "/path/to/project": {
///       "path": "/path/to/project",
///       "name": "project",
///       "last_accessed": 1234567890,
///       "access_count": 5,
///       "created_at": 1234567000
///     }
///   },
///   "sessions": [
///     {
///       "name": "session-name",
///       "project_path": "/path/to/project"
///     }
///   ]
/// }
/// ```
pub struct JsonStorage {
    /// Path to the JSON file on disk.
    file_path: PathBuf,

    /// In-memory data cache, loaded on creation.
    data: StorageData,

    /// Tracks if data has been modified since last save.
    dirty: bool,
}

impl JsonStorage {
    /// Creates or opens a JSON storage backend.
    ///
    /// If the file exists, loads existing data. Otherwise creates a new empty storage.
    /// Parent directories are created automatically.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Parent directory creation fails
    /// - File exists but contains invalid JSON
    /// - File permissions prevent reading
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zessionizer::storage::JsonStorage;
    /// use std::path::PathBuf;
    ///
    /// let storage = JsonStorage::new(PathBuf::from("/tmp/projects.json"))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(file_path: PathBuf) -> Result<Self> {
        tracing::debug!(path = ?file_path, "initializing JSON storage");

        if let Some(parent) = file_path.parent() {
            tracing::debug!(parent = ?parent, "creating parent directory");
            std::fs::create_dir_all(parent)?;
        }

        let data = if file_path.exists() {
            tracing::debug!("loading existing data");
            Self::load_from_file(&file_path)?
        } else {
            tracing::debug!("initializing new empty storage");
            StorageData::default()
        };

        tracing::debug!(
            project_count = data.projects.len(),
            session_count = data.sessions.len(),
            "storage initialized"
        );

        Ok(Self {
            file_path,
            data,
            dirty: false,
        })
    }

    /// Loads storage data from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid JSON.
    fn load_from_file(path: &PathBuf) -> Result<StorageData> {
        let contents = std::fs::read_to_string(path)?;
        let data: StorageData = serde_json::from_str(&contents)
            .map_err(|e| ZessionizerError::Storage(format!("failed to parse JSON: {e}")))?;

        tracing::debug!(
            version = data.version,
            projects = data.projects.len(),
            sessions = data.sessions.len(),
            "loaded storage data"
        );

        Ok(data)
    }

    /// Saves storage data to disk using atomic write.
    ///
    /// Writes to a temporary file first, then atomically renames it to the target path.
    /// This ensures the file is never left in a corrupt state, even if the process crashes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JSON serialization fails (should never happen with valid data)
    /// - Temporary file cannot be written
    /// - Rename operation fails (rare on POSIX systems)
    fn save_to_file(&mut self) -> Result<()> {
        if !self.dirty {
            tracing::trace!("skipping save, no changes");
            return Ok(());
        }

        tracing::debug!(path = ?self.file_path, "saving storage data");

        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| ZessionizerError::Storage(format!("failed to serialize JSON: {e}")))?;

        let tmp_path = self.file_path.with_extension("tmp");

        tracing::trace!(tmp_path = ?tmp_path, "writing to temporary file");
        std::fs::write(&tmp_path, json)?;

        tracing::trace!("renaming temporary file to final location");
        std::fs::rename(&tmp_path, &self.file_path)?;

        self.dirty = false;
        tracing::debug!("storage saved successfully");
        Ok(())
    }

    /// Returns the next available project ID.
    ///
    /// IDs are 1-indexed. Returns the count of projects + 1.
    fn next_project_id(&self) -> i64 {
        i64::try_from(self.data.projects.len())
            .unwrap_or(0)
            .saturating_add(1)
    }
}

impl Storage for JsonStorage {
    fn add_project(&mut self, project: &ProjectRecord) -> Result<i64> {
        let _span = tracing::debug_span!("json_add_project",
            project_path = %project.path,
            project_name = %project.name
        ).entered();

        let id = if let Some(existing) = self.data.projects.get_mut(&project.path) {
            tracing::debug!("updating existing project");
            existing.name.clone_from(&project.name);
            existing.last_accessed = project.last_accessed;
            existing.access_count = project.access_count;

            // Calculate ID based on position (not ideal but consistent with interface)
            i64::try_from(
                self.data.projects.keys()
                    .position(|k| k == &project.path)
                    .unwrap_or(0)
            ).unwrap_or(0) + 1
        } else {
            tracing::debug!("inserting new project");
            let id = self.next_project_id();
            self.data.projects.insert(project.path.clone(), project.clone());
            id
        };

        self.dirty = true;
        self.save_to_file()?;

        tracing::debug!(project_id = id, "project added");
        Ok(id)
    }

    fn add_projects_batch(&mut self, projects: &[ProjectRecord]) -> Result<Vec<ProjectRecord>> {
        let _span = tracing::debug_span!("json_add_projects_batch",
            count = projects.len()
        ).entered();

        let mut added = Vec::with_capacity(projects.len());

        for project in projects {
            if let Some(existing) = self.data.projects.get_mut(&project.path) {
                existing.name.clone_from(&project.name);
                existing.last_accessed = project.last_accessed;
                existing.access_count = existing.access_count.max(project.access_count);
                added.push(existing.clone());
            } else {
                self.data.projects.insert(project.path.clone(), project.clone());
                added.push(project.clone());
            }
        }

        self.dirty = true;
        self.save_to_file()?;

        tracing::debug!(added_count = added.len(), "batch added");
        Ok(added)
    }

    fn get_all_projects(&self) -> Result<Vec<ProjectRecord>> {
        let _span = tracing::debug_span!("json_get_all_projects").entered();

        let projects: Vec<ProjectRecord> = self.data.projects.values().cloned().collect();

        tracing::debug!(count = projects.len(), "retrieved projects");
        Ok(projects)
    }

    fn update_project_access(&mut self, path: &str, timestamp: i64) -> Result<()> {
        let _span = tracing::debug_span!("json_update_project_access",
            path = %path,
            timestamp = timestamp
        ).entered();

        let project = self.data.projects.get_mut(path)
            .ok_or_else(|| ZessionizerError::Storage(format!("project not found: {path}")))?;

        project.last_accessed = Some(timestamp);
        project.access_count = project.access_count.saturating_add(1);
        let new_count = project.access_count;

        self.dirty = true;
        self.save_to_file()?;

        tracing::debug!(
            new_count = new_count,
            "project access updated"
        );
        Ok(())
    }

    fn get_project_by_path(&self, path: &str) -> Result<Option<ProjectRecord>> {
        let _span = tracing::debug_span!("json_get_project_by_path",
            path = %path
        ).entered();

        let project = self.data.projects.get(path).cloned();

        tracing::debug!(found = project.is_some(), "project lookup complete");
        Ok(project)
    }

    fn get_all_sessions(&self) -> Result<Vec<SessionRecord>> {
        let _span = tracing::debug_span!("json_get_all_sessions").entered();

        let sessions = self.data.sessions.clone();

        tracing::debug!(count = sessions.len(), "retrieved sessions");
        Ok(sessions)
    }

    fn sync_sessions(&mut self, active_session_names: &[String]) -> Result<()> {
        let _span = tracing::debug_span!("json_sync_sessions",
            active_count = active_session_names.len()
        ).entered();

        // Clear old sessions and rebuild from active list
        self.data.sessions.clear();

        // For each active session, try to find matching project
        for session_name in active_session_names {
            if let Some(project) = self.data.projects.values()
                .find(|p| p.name == *session_name)
            {
                self.data.sessions.push(SessionRecord {
                    name: session_name.clone(),
                    project_path: project.path.clone(),
                });
            }
        }

        self.dirty = true;
        self.save_to_file()?;

        tracing::debug!(
            synced_count = self.data.sessions.len(),
            "sessions synced"
        );
        Ok(())
    }
}

impl Drop for JsonStorage {
    /// Ensures data is saved on drop, even if the user forgot to call save explicitly.
    fn drop(&mut self) {
        if self.dirty {
            tracing::debug!("saving dirty data on drop");
            if let Err(e) = self.save_to_file() {
                tracing::error!(error = %e, "failed to save on drop");
            }
        }
    }
}
