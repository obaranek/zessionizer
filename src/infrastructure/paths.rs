//! Path helpers for the Zellij plugin sandbox.
//!
//! Zellij plugins run in a WASI sandbox where the host filesystem is mounted
//! under `/host`. In practice `/host` corresponds to the cwd of the last
//! focused terminal pane (or the directory Zellij was launched from), which
//! is typically the user's home directory.
//!
//! User-facing paths use the conventional `~/` prefix; in-sandbox paths used
//! when invoking host commands (notably `find`) must use the `/host/` prefix.
//! The helpers below convert between the two forms.

use std::path::PathBuf;

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
/// Paths that are already absolute (or that don't begin with `~`) are returned
/// unchanged.
#[must_use]
pub fn to_sandbox_path(path: &str) -> String {
    if path == "~" {
        return "/host".to_string();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return format!("/host/{rest}");
    }
    path.to_string()
}

/// Converts an in-sandbox path (`/host/...`) to its user-visible form (`~/...`).
///
/// Paths that aren't under `/host` are returned unchanged.
#[must_use]
pub fn from_sandbox_path(path: &str) -> String {
    if path == "/host" {
        return "~".to_string();
    }
    if let Some(rest) = path.strip_prefix("/host/") {
        return format!("~/{rest}");
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
