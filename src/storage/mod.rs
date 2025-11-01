//! Storage layer for persistent project and session data.
//!
//! This module provides the storage abstraction for persisting project information,
//! tracking access patterns, and managing Zellij session associations. It uses
//! JSON file storage with frecency-based sorting for project lists.
//!
//! # Modules
//!
//! - `backend`: Storage trait abstraction for backend implementations
//! - `json`: JSON file-based storage implementation
//! - `frecency`: Scoring algorithm combining frequency and recency
//! - `models`: Storage record types separate from domain models

pub mod backend;
pub mod frecency;
pub mod json;
pub mod models;

pub use backend::Storage;
pub use frecency::{calculate_score, sort_by_frecency};
pub use json::JsonStorage;
pub use models::{ProjectRecord, SessionRecord};
