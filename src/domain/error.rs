//! Error types for the Zessionizer plugin.
//!
//! This module defines the centralized error type [`ZessionizerError`] and a type alias
//! [`Result`] for convenient error handling throughout the plugin. All errors are
//! implemented using the `thiserror` crate for automatic `Error` trait implementation.

use thiserror::Error;

/// The main error type for Zessionizer plugin operations.
///
/// This enum consolidates all error conditions that can occur during plugin execution,
/// from storage operations to I/O failures and configuration issues. Most variants
/// wrap underlying errors from external crates using `#[from]` for automatic conversion.
///
/// # Examples
///
/// ```
/// use crate::domain::ZessionizerError;
///
/// // Explicit error construction
/// fn validate_config() -> Result<(), ZessionizerError> {
///     Err(ZessionizerError::Config("Missing required field".to_string()))
/// }
///
/// fn read_storage() -> Result<(), ZessionizerError> {
///     Err(ZessionizerError::Storage("Failed to read file".to_string()))
/// }
/// ```
#[derive(Debug, Error)]
pub enum ZessionizerError {
    /// Storage operation failed.
    ///
    /// Occurs when reading from or writing to the storage backend fails.
    /// The string contains a description of what went wrong.
    #[error("Storage error: {0}")]
    Storage(String),

    /// Filesystem or I/O operation failed.
    ///
    /// Wraps errors from standard library I/O operations. Automatically converts
    /// from `std::io::Error` using the `#[from]` attribute.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Theme parsing or application failed.
    ///
    /// Occurs when the plugin cannot parse or apply the configured Zellij theme.
    /// The string contains a description of what went wrong.
    #[error("Theme error: {0}")]
    Theme(String),

    /// Communication with background worker failed.
    ///
    /// Occurs when the plugin cannot communicate with its background worker thread,
    /// typically during project scanning or storage operations. The string contains
    /// details about the communication failure.
    #[error("Worker communication error: {0}")]
    Worker(String),

    /// Configuration is invalid or missing.
    ///
    /// Occurs when required configuration values are missing or malformed.
    /// The string describes the specific configuration problem.
    #[error("Configuration error: {0}")]
    Config(String),
}

/// A specialized `Result` type for Zessionizer operations.
///
/// This is a type alias for `std::result::Result<T, ZessionizerError>` that simplifies
/// function signatures throughout the codebase.
///
/// # Examples
///
/// ```
/// use crate::domain::Result;
///
/// fn process_project() -> Result<()> {
///     // Function that may return ZessionizerError
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, ZessionizerError>;
