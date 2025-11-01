//! Path manipulation utilities for Zellij sandbox environment.
//!
//! This module provides functions for working with filesystem paths in the Zellij
//! plugin sandbox, where the host filesystem is mounted under `/host`. It handles
//! tilde expansion, path normalization, and storage location management.

use std::path::PathBuf;

/// Returns the data directory for Zessionizer storage.
///
/// The directory is located at `/host/.local/share/zellij/zessionizer` in the Zellij
/// sandbox. In Zellij's plugin environment, `/host` points to the cwd of the last
/// focused terminal, or the folder where Zellij was started if that's not available.
///
/// This typically resolves to the user's home directory when Zellij is started from
/// a home directory terminal, making the actual path `~/.local/share/zellij/zessionizer`.
/// The JSON storage file `projects.json` is located within this directory.
///
/// # Examples
///
/// ```
/// use crate::infrastructure::get_data_dir;
///
/// let data_dir = get_data_dir();
/// assert_eq!(data_dir.to_str().unwrap(), "/host/.local/share/zellij/zessionizer");
/// ```
#[must_use]
pub fn get_data_dir() -> PathBuf {
    PathBuf::from("/host/.local/share/zellij").join("zessionizer")
}

/// Expands tilde paths to use the `/host` prefix for Zellij sandbox.
///
/// In the Zellij sandbox environment, the host's home directory (`~`) maps to `/host`.
/// This function converts tilde-prefixed paths to their sandbox equivalents.
///
/// # Examples
///
/// ```
/// use crate::infrastructure::expand_tilde;
///
/// assert_eq!(expand_tilde("~/projects"), "/host/projects");
/// assert_eq!(expand_tilde("~"), "/host");
/// assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
/// ```
#[must_use]
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        path.replacen('~', "/host", 1)
    } else if path == "~" {
        "/host".to_string()
    } else {
        path.to_string()
    }
}

/// Removes the `/host` prefix from sandbox paths for display purposes.
///
/// When showing paths to users, it's often clearer to remove the sandbox prefix
/// so paths appear as they would on the host filesystem.
///
/// # Examples
///
/// ```
/// use crate::infrastructure::strip_host_prefix;
///
/// assert_eq!(strip_host_prefix("/host/home/user/project"), "/home/user/project");
/// assert_eq!(strip_host_prefix("/absolute/path"), "/absolute/path");
/// ```
#[must_use]
pub fn strip_host_prefix(path: &str) -> String {
    path.strip_prefix("/host")
        .unwrap_or(path)
        .to_string()
}
