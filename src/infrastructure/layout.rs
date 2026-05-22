//! Layout resolution for project sessions.
//!
//! When the user opens a project, the plugin chooses which Zellij layout to
//! apply via the following precedence:
//!
//! 1. `<project>/.zessionizer.kdl` — per-project override committed alongside
//!    the repo.
//! 2. The plugin-wide `default_project_layout` config value, treated as a path
//!    to a KDL file.
//! 3. The embedded built-in layout ([`BUILTIN_LAYOUT`]), a minimal single-pane
//!    layout that defers to Zellij's defaults.
//!
//! [`resolve_for`] returns `Some(path)` in **user-visible (`~/...`) form** when
//! an external KDL file is selected, and `None` to signal "use the built-in."
//! The plugin's filesystem existence checks happen internally via the
//! `/host/...` sandbox form, but the returned path is always user-form so the
//! caller can hand it to whichever boundary expects what (Zellij APIs,
//! display, storage).

use crate::infrastructure::to_sandbox_path;
use std::path::PathBuf;

/// KDL layout used when neither the project nor the config specifies one.
///
/// A minimal single-pane layout — opens the session with one pane and lets
/// Zellij pick up whatever tab template / status bar the user has globally
/// configured.
pub const BUILTIN_LAYOUT: &str = include_str!("../../assets/default_project_layout.kdl");

/// Filename the plugin looks for inside a project to override the default
/// layout.
pub const PROJECT_LAYOUT_FILENAME: &str = ".zessionizer.kdl";

/// Resolves which layout to apply when opening `project_path`.
///
/// Both `project_path` and `default` use the user-visible (`~/...` or absolute)
/// form. Filesystem existence checks happen internally against the in-sandbox
/// (`/host/...`) form. The returned path retains the original user-visible
/// form so the caller can apply a single boundary expansion at dispatch.
///
/// Returns `Some(user-form path)` when an external KDL file should be used
/// and `None` to indicate the caller should fall back to [`BUILTIN_LAYOUT`].
#[must_use]
pub fn resolve_for(project_path: &str, default: Option<&str>) -> Option<PathBuf> {
    let project_kdl_sandbox =
        PathBuf::from(to_sandbox_path(project_path)).join(PROJECT_LAYOUT_FILENAME);
    if project_kdl_sandbox.is_file() {
        return Some(join_user_path(project_path, PROJECT_LAYOUT_FILENAME));
    }

    let default_path = default?;
    let default_sandbox = PathBuf::from(to_sandbox_path(default_path));
    if default_sandbox.is_file() {
        return Some(PathBuf::from(default_path));
    }

    tracing::debug!(
        default = ?default,
        sandbox = %default_sandbox.display(),
        "default_project_layout is set but the file does not exist; falling back to builtin",
    );
    None
}

/// Joins a child filename onto a user-visible path, preserving its form so
/// `~/foo` stays `~/foo/.zessionizer.kdl` (rather than becoming an absolute
/// path the way `Path::join` would treat tildes).
fn join_user_path(base: &str, filename: &str) -> PathBuf {
    PathBuf::from(format!("{}/{filename}", base.trim_end_matches('/')))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_kdl(dir: &std::path::Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "layout {}\n").unwrap();
        path
    }

    #[test]
    fn project_layout_wins_over_config_default() {
        let project_dir = TempDir::new().unwrap();
        let project_kdl = write_kdl(project_dir.path(), PROJECT_LAYOUT_FILENAME);

        let config_dir = TempDir::new().unwrap();
        let config_kdl = write_kdl(config_dir.path(), "team.kdl");

        let project_path = project_dir.path().to_str().unwrap();
        let default = Some(config_kdl.to_str().unwrap());

        // Resolver returns the project path (input form preserved); since the
        // input is already absolute, the returned path equals project_kdl.
        assert_eq!(resolve_for(project_path, default), Some(project_kdl));
    }

    #[test]
    fn config_default_used_when_project_layout_missing() {
        let project_dir = TempDir::new().unwrap();

        let config_dir = TempDir::new().unwrap();
        let config_kdl = write_kdl(config_dir.path(), "team.kdl");

        let project_path = project_dir.path().to_str().unwrap();
        let default = Some(config_kdl.to_str().unwrap());

        assert_eq!(resolve_for(project_path, default), Some(config_kdl));
    }

    #[test]
    fn falls_back_to_builtin_when_nothing_is_set() {
        let project_dir = TempDir::new().unwrap();
        let project_path = project_dir.path().to_str().unwrap();

        assert_eq!(resolve_for(project_path, None), None);
    }

    #[test]
    fn falls_back_to_builtin_when_default_path_is_missing() {
        let project_dir = TempDir::new().unwrap();
        let project_path = project_dir.path().to_str().unwrap();

        assert_eq!(
            resolve_for(project_path, Some("/definitely/not/here.kdl")),
            None
        );
    }

    #[test]
    fn returned_path_preserves_user_form_for_tilde_input() {
        // Real `find` output is in `~/Foo` form (post-`from_sandbox_path`);
        // when a per-project layout is found, the resolver should return a
        // matching `~/Foo/.zessionizer.kdl`, not a `/host/Foo/...` form.
        // We can't actually create `~/Foo` in a unit test, so we exercise
        // the helper that does the join directly.
        let joined = join_user_path("~/Projects/foo", PROJECT_LAYOUT_FILENAME);
        assert_eq!(joined, PathBuf::from("~/Projects/foo/.zessionizer.kdl"));

        let joined_trailing = join_user_path("~/Projects/foo/", PROJECT_LAYOUT_FILENAME);
        assert_eq!(
            joined_trailing,
            PathBuf::from("~/Projects/foo/.zessionizer.kdl"),
        );
    }

    #[test]
    fn builtin_layout_is_non_empty() {
        assert!(!BUILTIN_LAYOUT.trim().is_empty());
        assert!(BUILTIN_LAYOUT.contains("layout"));
    }
}
