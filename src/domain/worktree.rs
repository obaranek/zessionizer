//! Worktree domain model.
//!
//! A `Worktree` represents a checkout of a Git repository -- either the trunk
//! (the project's primary working tree) or one of the sibling worktrees living
//! under `<project>/.worktrees/`. Worktrees are derived from the filesystem at
//! pick-time and are not persisted.

use serde::{Deserialize, Serialize};

/// Display name used for the project's primary checkout.
pub const TRUNK_NAME: &str = "trunk";

/// A single Git worktree, surfaced in the picker alongside its siblings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worktree {
    /// User-visible filesystem path (`~/...` form) to the worktree's working
    /// directory.
    pub path: String,

    /// Display name. For trunk this is [`TRUNK_NAME`]; for sibling worktrees
    /// it's the directory name inside `.worktrees/`.
    pub name: String,

    /// Whether this entry is the project's trunk checkout.
    pub is_trunk: bool,
}
