//! Filesystem scan for a project's worktrees.
//!
//! A project that uses Git worktrees keeps its sibling checkouts under
//! `<project>/.worktrees/<name>`. [`scan`] walks that directory and returns
//! one [`Worktree`] per subdir, plus a synthetic trunk entry for the project
//! root checkout itself.
//!
//! All paths are user-visible (`~/...`); the resolver translates internally
//! to the in-sandbox `/host/...` form when touching the filesystem.

use crate::domain::worktree::{Worktree, TRUNK_NAME};
use crate::infrastructure::{from_sandbox_path, to_sandbox_path};
use std::path::{Path, PathBuf};

/// Subdirectory name that holds sibling worktrees.
pub const WORKTREES_DIRNAME: &str = ".worktrees";

/// Returns true if the project at `project_path` (user-visible form) contains
/// a `.worktrees/` directory.
#[must_use]
pub fn has_worktrees_dir(project_path: &str) -> bool {
    let dir = PathBuf::from(to_sandbox_path(project_path)).join(WORKTREES_DIRNAME);
    dir.is_dir()
}

/// Scans the worktrees of the project at `project_path` (user-visible form).
///
/// Returns the trunk entry first, then one entry per subdirectory of
/// `<project>/.worktrees/`, sorted lexicographically by name. If the
/// `.worktrees/` directory is absent or unreadable, only the trunk entry is
/// returned.
#[must_use]
pub fn scan(project_path: &str) -> Vec<Worktree> {
    let trunk_name = trunk_name_for(project_path);
    let mut out = vec![Worktree {
        path: project_path.to_string(),
        name: trunk_name,
        is_trunk: true,
    }];

    let dir = PathBuf::from(to_sandbox_path(project_path)).join(WORKTREES_DIRNAME);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };

    let mut subs: Vec<Worktree> = entries
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            if name.starts_with('.') {
                return None;
            }
            let sandbox_path = e.path();
            let user_path = from_sandbox_path(sandbox_path.to_str()?);
            Some(Worktree {
                path: user_path,
                name,
                is_trunk: false,
            })
        })
        .collect();

    subs.sort_by(|a, b| a.name.cmp(&b.name));
    out.extend(subs);
    out
}

/// Builds a display name for the trunk entry. Defaults to [`TRUNK_NAME`] but
/// falls back gracefully if the project path is malformed.
fn trunk_name_for(_project_path: &str) -> String {
    TRUNK_NAME.to_string()
}

/// Returns the project's directory name from a path, used as the session
/// name when opening any worktree of that project.
#[must_use]
pub fn session_name_from_project(project_path: &str) -> Option<String> {
    Path::new(project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn scan_returns_trunk_only_when_no_worktrees_dir() {
        let dir = TempDir::new().unwrap();
        let result = scan(dir.path().to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert!(result[0].is_trunk);
        assert_eq!(result[0].name, TRUNK_NAME);
    }

    #[test]
    fn scan_lists_worktree_subdirectories() {
        let dir = TempDir::new().unwrap();
        let worktrees = dir.path().join(WORKTREES_DIRNAME);
        fs::create_dir(&worktrees).unwrap();
        fs::create_dir(worktrees.join("feat-z")).unwrap();
        fs::create_dir(worktrees.join("feat-a")).unwrap();

        let result = scan(dir.path().to_str().unwrap());
        let names: Vec<&str> = result.iter().map(|w| w.name.as_str()).collect();

        assert_eq!(names, vec![TRUNK_NAME, "feat-a", "feat-z"]);
        assert!(result[0].is_trunk);
        assert!(!result[1].is_trunk);
        assert!(!result[2].is_trunk);
    }

    #[test]
    fn scan_ignores_files_and_dotted_entries() {
        let dir = TempDir::new().unwrap();
        let worktrees = dir.path().join(WORKTREES_DIRNAME);
        fs::create_dir(&worktrees).unwrap();
        fs::create_dir(worktrees.join("feat-x")).unwrap();
        fs::create_dir(worktrees.join(".hidden")).unwrap();
        fs::write(worktrees.join("README"), "").unwrap();

        let result = scan(dir.path().to_str().unwrap());
        let names: Vec<&str> = result.iter().map(|w| w.name.as_str()).collect();

        assert_eq!(names, vec![TRUNK_NAME, "feat-x"]);
    }

    #[test]
    fn has_worktrees_dir_detects_directory() {
        let dir = TempDir::new().unwrap();
        assert!(!has_worktrees_dir(dir.path().to_str().unwrap()));

        fs::create_dir(dir.path().join(WORKTREES_DIRNAME)).unwrap();
        assert!(has_worktrees_dir(dir.path().to_str().unwrap()));
    }

    #[test]
    fn session_name_from_project_uses_basename() {
        assert_eq!(
            session_name_from_project("~/Projects/foo").as_deref(),
            Some("foo"),
        );
        assert_eq!(
            session_name_from_project("/home/user/Code/bar").as_deref(),
            Some("bar"),
        );
    }
}
