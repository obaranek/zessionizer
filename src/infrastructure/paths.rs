//! Path helpers for the Zellij plugin sandbox.
//!
//! Zellij plugins run in a WASI sandbox where the host filesystem is mounted
//! under `/host`. In practice `/host` corresponds to the cwd of the last
//! focused terminal pane (or the directory Zellij was launched from), which
//! is typically the user's home directory.
//!
//! Three path forms appear in this codebase:
//! - **User-visible** (`~/Projects/foo`): how paths are stored and displayed.
//! - **Sandbox** (`/host/Projects/foo`): used when invoking host commands
//!   (notably `find`) and when the plugin reads/writes the host filesystem.
//! - **Absolute host** (`/Users/alice/Projects/foo`): required by Zellij APIs
//!   that run *outside* the sandbox (notably `switch_session_with_layout`,
//!   which receives the cwd via the protobuf bridge and resolves it on the
//!   host -- where `~` and `/host` are both meaningless).
//!
//! [`to_sandbox_path`] and [`from_sandbox_path`] swap between the first two.
//! [`expand_to_host_path`] takes a user-visible path and the host's home
//! directory (sourced once at plugin load via `get_plugin_ids().initial_cwd`)
//! and produces the third.

use std::path::{Path, PathBuf};

const TILDE: &str = "~";
const TILDE_PREFIX: &str = "~/";
const SANDBOX_ROOT: &str = "/host";
const SANDBOX_PREFIX: &str = "/host/";

/// Returns the data directory for Zessionizer storage.
///
/// Located at `/host/.local/share/zellij/zessionizer`, i.e.
/// `~/.local/share/zellij/zessionizer` on the host.
#[must_use]
pub fn get_data_dir() -> PathBuf {
    PathBuf::from("/host/.local/share/zellij").join("zessionizer")
}

/// Converts a user-visible path (`~/...`) to its in-sandbox form (`/host/...`).
///
/// Paths that don't begin with `~` are returned unchanged. Note that this
/// means **bare relative paths like `Projects/foo`** are passed through as-is;
/// `find Projects/foo` will only resolve when the sandbox cwd happens to be
/// the host's home. Configure `scan_paths` with leading `~/` (or absolute
/// paths) to be explicit.
#[must_use]
pub fn to_sandbox_path(path: &str) -> String {
    if path == TILDE {
        return SANDBOX_ROOT.to_string();
    }
    if let Some(rest) = path.strip_prefix(TILDE_PREFIX) {
        return format!("{SANDBOX_PREFIX}{rest}");
    }
    path.to_string()
}

/// Converts an in-sandbox path (`/host/...`) to its user-visible form (`~/...`).
///
/// Paths that aren't under `/host` are returned unchanged.
#[must_use]
pub fn from_sandbox_path(path: &str) -> String {
    if path == SANDBOX_ROOT {
        return TILDE.to_string();
    }
    if let Some(rest) = path.strip_prefix(SANDBOX_PREFIX) {
        return format!("{TILDE_PREFIX}{rest}");
    }
    path.to_string()
}

/// Converts an absolute host path back to its user-visible (`~/...`) form by
/// stripping the host's home-directory prefix. Paths that don't sit under
/// `host_home` are returned unchanged.
///
/// This is the inverse of [`expand_to_host_path`]'s `~/` branch and is used
/// to normalize `find` output (which produces absolute host paths because the
/// `find` process runs outside the WASI sandbox) before paths flow through
/// the rest of the system.
#[must_use]
pub fn from_host_path(path: &str, host_home: &Path) -> String {
    let host_home_str = host_home.to_string_lossy();
    let trimmed = host_home_str.trim_end_matches('/');
    if trimmed.is_empty() {
        return path.to_string();
    }
    if path == trimmed {
        return TILDE.to_string();
    }
    if let Some(rest) = path.strip_prefix(trimmed) {
        if let Some(suffix) = rest.strip_prefix('/') {
            return format!("{TILDE_PREFIX}{suffix}");
        }
    }
    path.to_string()
}

/// Resolves a user-visible path against the host's home directory, producing
/// an absolute host path suitable for Zellij APIs that execute outside the
/// sandbox.
///
/// `host_home` should be the value of `get_plugin_ids().initial_cwd` -- the
/// directory that maps to `/host` inside the sandbox. The conversions are:
///
/// | Input               | Output                          |
/// |---------------------|---------------------------------|
/// | `~`                 | `<host_home>`                   |
/// | `~/Projects/foo`    | `<host_home>/Projects/foo`      |
/// | `/host`             | `<host_home>`                   |
/// | `/host/Projects/x`  | `<host_home>/Projects/x`        |
/// | `/Users/alice/...`  | unchanged (already absolute)    |
/// | `Projects/foo`      | `<host_home>/Projects/foo`      |
#[must_use]
pub fn expand_to_host_path(path: &str, host_home: &Path) -> PathBuf {
    if path == TILDE || path == SANDBOX_ROOT {
        return host_home.to_path_buf();
    }
    if let Some(rest) = path
        .strip_prefix(TILDE_PREFIX)
        .or_else(|| path.strip_prefix(SANDBOX_PREFIX))
    {
        return host_home.join(rest);
    }
    if path.starts_with('/') {
        return PathBuf::from(path);
    }
    host_home.join(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn to_sandbox_expands_tilde_with_subpath() {
        assert_eq!(to_sandbox_path("~/Projects"), "/host/Projects");
        assert_eq!(to_sandbox_path("~/Projects/foo"), "/host/Projects/foo");
    }

    #[test]
    fn to_sandbox_expands_bare_tilde() {
        assert_eq!(to_sandbox_path("~"), "/host");
    }

    #[test]
    fn to_sandbox_passes_through_absolute_paths() {
        assert_eq!(to_sandbox_path("/usr/local/bin"), "/usr/local/bin");
        assert_eq!(to_sandbox_path("/host/Projects"), "/host/Projects");
    }

    #[test]
    fn to_sandbox_does_not_touch_tilde_in_middle() {
        assert_eq!(to_sandbox_path("/some/~weird/path"), "/some/~weird/path");
    }

    #[test]
    fn from_sandbox_strips_host_prefix() {
        assert_eq!(from_sandbox_path("/host/Projects"), "~/Projects");
        assert_eq!(from_sandbox_path("/host/Projects/foo"), "~/Projects/foo");
    }

    #[test]
    fn from_sandbox_handles_bare_host() {
        assert_eq!(from_sandbox_path("/host"), "~");
    }

    #[test]
    fn from_sandbox_passes_through_unrelated_paths() {
        assert_eq!(from_sandbox_path("/usr/local/bin"), "/usr/local/bin");
        assert_eq!(from_sandbox_path("~/Projects"), "~/Projects");
    }

    #[test]
    fn from_sandbox_does_not_strip_partial_match() {
        // "/hostile" must not be treated as "/host" + "ile".
        assert_eq!(from_sandbox_path("/hostile"), "/hostile");
    }

    #[test]
    fn round_trip_user_to_sandbox_to_user() {
        for input in ["~/Projects", "~/Projects/foo/bar", "~"] {
            assert_eq!(from_sandbox_path(&to_sandbox_path(input)), input);
        }
    }

    #[test]
    fn expand_resolves_tilde_form() {
        let home = Path::new("/Users/alice");
        assert_eq!(
            expand_to_host_path("~/Projects/foo", home),
            PathBuf::from("/Users/alice/Projects/foo"),
        );
        assert_eq!(expand_to_host_path("~", home), PathBuf::from("/Users/alice"));
    }

    #[test]
    fn expand_resolves_sandbox_form() {
        let home = Path::new("/Users/alice");
        assert_eq!(
            expand_to_host_path("/host/Projects/foo", home),
            PathBuf::from("/Users/alice/Projects/foo"),
        );
        assert_eq!(
            expand_to_host_path("/host", home),
            PathBuf::from("/Users/alice"),
        );
    }

    #[test]
    fn expand_passes_through_absolute_host_paths() {
        let home = Path::new("/Users/alice");
        assert_eq!(
            expand_to_host_path("/etc/hosts", home),
            PathBuf::from("/etc/hosts"),
        );
    }

    #[test]
    fn expand_treats_relative_as_relative_to_home() {
        // Legacy projects.json entries (pre-rebuild) stored paths like
        // `Projects/foo` after the old strip_prefix. Treat them as
        // home-relative so anyone migrating doesn't get a broken cwd.
        let home = Path::new("/Users/alice");
        assert_eq!(
            expand_to_host_path("Projects/foo", home),
            PathBuf::from("/Users/alice/Projects/foo"),
        );
    }

    #[test]
    fn expand_does_not_strip_partial_host_match() {
        let home = Path::new("/Users/alice");
        // "/hostile" must not be rewritten as "/Users/alice/ile".
        assert_eq!(
            expand_to_host_path("/hostile", home),
            PathBuf::from("/hostile"),
        );
    }

    #[test]
    fn from_host_strips_home_prefix() {
        let home = Path::new("/Users/alice");
        assert_eq!(from_host_path("/Users/alice/Git/foo", home), "~/Git/foo");
        assert_eq!(from_host_path("/Users/alice", home), "~");
    }

    #[test]
    fn from_host_handles_trailing_slash_in_home() {
        let home = Path::new("/Users/alice/");
        assert_eq!(from_host_path("/Users/alice/Git/foo", home), "~/Git/foo");
        assert_eq!(from_host_path("/Users/alice", home), "~");
    }

    #[test]
    fn from_host_passes_through_unrelated_paths() {
        let home = Path::new("/Users/alice");
        assert_eq!(from_host_path("/etc/hosts", home), "/etc/hosts");
        assert_eq!(from_host_path("~/already-user", home), "~/already-user");
    }

    #[test]
    fn from_host_does_not_match_partial_prefix() {
        let home = Path::new("/Users/al");
        // "/Users/alice/X" must not be matched against "/Users/al" + "ice/X".
        assert_eq!(from_host_path("/Users/alice/Git", home), "/Users/alice/Git");
    }
}
