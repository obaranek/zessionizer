//! Infrastructure layer for filesystem and environment interactions.
//!
//! This module provides utilities for working with the Zellij plugin sandbox
//! environment, particularly path handling where the host filesystem is mounted
//! under `/host`.

pub mod layout;
pub mod paths;
pub mod worktree;

pub use paths::{expand_to_host_path, from_host_path, from_sandbox_path, get_data_dir, to_sandbox_path};
