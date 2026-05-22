//! Infrastructure layer for filesystem and environment interactions.
//!
//! This module provides utilities for working with the Zellij plugin sandbox
//! environment, particularly path handling where the host filesystem is mounted
//! under `/host`.

pub mod paths;

pub use paths::{from_sandbox_path, get_data_dir, to_sandbox_path};
