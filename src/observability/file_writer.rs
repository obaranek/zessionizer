//! Rotating file writer with size-based rotation and backup retention.
//!
//! This module provides a thread-safe file writer that automatically rotates
//! files when they exceed a size threshold, maintaining a fixed number of
//! backup files. This prevents unbounded disk usage for trace files.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Maximum file size before rotation (10 MB).
const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;

/// Number of backup files to retain after rotation.
const MAX_BACKUP_FILES: usize = 3;

/// Thread-safe rotating file writer.
///
/// Provides automatic file rotation based on size thresholds. When the current
/// file exceeds `MAX_FILE_SIZE_BYTES`, it is renamed with a timestamp suffix
/// and a new file is created. Old backups beyond `MAX_BACKUP_FILES` are
/// automatically cleaned up.
///
/// # Thread Safety
///
/// Uses an internal `Mutex` to ensure safe concurrent access. Multiple threads
/// can safely write to the same `FileWriter` instance.
///
/// # Rotation Strategy
///
/// 1. Check file size before each write
/// 2. If size > 10MB, rotate:
///    - Rename current file to `<name>.json.<timestamp>`
///    - Create new empty file
///    - Remove oldest backups beyond 3
///
/// # Example
///
/// ```rust
/// use std::path::PathBuf;
///
/// let path = PathBuf::from("/tmp/traces.json");
/// let writer = FileWriter::new(path);
///
/// // Writes are automatically rotated when file grows too large
/// writer.write_line("{\"trace\": \"data\"}").unwrap();
/// ```
pub struct FileWriter {
    /// Path to the primary log file.
    file_path: PathBuf,
    /// Lazily-initialized file handle (opens on first write).
    writer: Mutex<Option<std::fs::File>>,
}

impl FileWriter {
    /// Creates a new file writer for the given path.
    ///
    /// The file is not opened until the first write operation. This allows
    /// construction to succeed even if the file cannot be opened immediately.
    ///
    /// # Parameters
    ///
    /// * `file_path` - Path to the log file (will be created if it doesn't exist)
    pub const fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            writer: Mutex::new(None),
        }
    }

    /// Writes a single line to the file with automatic rotation.
    ///
    /// Checks file size before writing and rotates if necessary. The line is
    /// written with a trailing newline and flushed to disk immediately.
    ///
    /// # Parameters
    ///
    /// * `json` - JSON string to write (newline will be added)
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the write succeeded
    /// - `Err(io::Error)` if rotation, opening, writing, or flushing failed
    ///
    /// # Errors
    ///
    /// May fail due to:
    /// - File system permissions
    /// - Disk space exhaustion
    /// - Mutex poisoning (if another thread panicked while holding the lock)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use std::path::PathBuf;
    /// # let writer = FileWriter::new(PathBuf::from("/tmp/test.json"));
    /// writer.write_line("{\"event\": \"test\"}").unwrap();
    /// ```
    pub fn write_line(&self, json: &str) -> std::io::Result<()> {
        let mut writer = self.writer.lock().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Mutex poisoned: {e}"))
        })?;

        self.check_and_rotate(&mut writer)?;

        if writer.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)?;
            *writer = Some(file);
        }

        let file = writer.as_mut().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "No file available")
        })?;

        writeln!(file, "{json}")?;
        file.flush()?;
        drop(writer);

        Ok(())
    }

    /// Checks file size and rotates if necessary.
    ///
    /// If the current file exceeds `MAX_FILE_SIZE_BYTES`, closes the file
    /// handle and triggers rotation.
    ///
    /// # Parameters
    ///
    /// * `writer` - Current file handle (set to `None` if rotation occurs)
    fn check_and_rotate(&self, writer: &mut Option<std::fs::File>) -> std::io::Result<()> {
        if let Ok(metadata) = fs::metadata(&self.file_path) {
            if metadata.len() > MAX_FILE_SIZE_BYTES {
                *writer = None;
                self.rotate_files()?;
            }
        }
        Ok(())
    }

    /// Rotates the current file and cleans up old backups.
    ///
    /// Creates a timestamped backup of the current file and removes backups
    /// beyond the retention limit.
    ///
    /// # Backup Naming
    ///
    /// Backups are named: `<original_name>.json.<unix_timestamp>`
    ///
    /// Example: `zessionizer-otlp.json.1234567890`
    fn rotate_files(&self) -> std::io::Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();

        let backup_path = self.file_path.with_extension(format!("json.{timestamp}"));

        if self.file_path.exists() {
            fs::rename(&self.file_path, &backup_path)?;
        }

        self.cleanup_old_backups()?;

        Ok(())
    }

    /// Removes old backup files beyond the retention limit.
    ///
    /// Scans the directory for backup files matching the pattern
    /// `<name>.json.*`, sorts by modification time (newest first), and deletes
    /// all backups beyond `MAX_BACKUP_FILES`.
    ///
    /// # Error Handling
    ///
    /// Ignores individual file deletion errors to ensure cleanup continues even
    /// if some files cannot be removed.
    fn cleanup_old_backups(&self) -> std::io::Result<()> {
        let parent_dir = self.file_path.parent().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "No parent directory")
        })?;

        let file_stem = self.file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "Invalid file name")
            })?;

        // Find all backup files
        let mut backups: Vec<PathBuf> = fs::read_dir(parent_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(file_stem) && name.contains(".json."))
            })
            .collect();

        backups.sort_by(|a, b| {
            let a_time = fs::metadata(a).and_then(|m| m.modified()).ok();
            let b_time = fs::metadata(b).and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        // Remove backups beyond retention limit
        for old_backup in backups.iter().skip(MAX_BACKUP_FILES) {
            let _ = fs::remove_file(old_backup);
        }

        Ok(())
    }
}

impl std::fmt::Debug for FileWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileWriter")
            .field("file_path", &self.file_path)
            .finish_non_exhaustive()
    }
}
