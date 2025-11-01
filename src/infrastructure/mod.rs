//! Infrastructure layer for filesystem and environment interactions.
//!
//! This module provides utilities for working with the Zellij plugin sandbox
//! environment, particularly path handling where the host filesystem is mounted
//! under `/host`.

pub mod paths;

pub use paths::{expand_tilde, get_data_dir, strip_host_prefix};
