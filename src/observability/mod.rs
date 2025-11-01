//! OpenTelemetry-based observability with file-based trace export.
//!
//! This module provides distributed tracing infrastructure for the plugin,
//! using OpenTelemetry OTLP format with file-based exporting. Traces are
//! written to JSON files for offline analysis and debugging.
//!
//! # Architecture
//!
//! The observability layer implements a custom file-based OTLP exporter:
//!
//! ```text
//! tracing-opentelemetry → OpenTelemetry SDK → FileSpanExporter → JSON Files
//! ```
//!
//! # Features
//!
//! - **File-Based Export**: Traces written to `~/.local/share/zellij/zessionizer/zessionizer-otlp.json`
//! - **Automatic Rotation**: Files rotate at 10MB with 3-backup retention
//! - **OTLP Format**: Standard OpenTelemetry Protocol JSON format
//! - **Resource Metadata**: Includes service name and environment info
//!
//! # Configuration
//!
//! Trace level is controlled via:
//! 1. `RUST_LOG` environment variable (highest priority)
//! 2. `trace_level` config option in plugin configuration
//! 3. Default: `"info"`
//!
//! # Usage
//!
//! Initialize tracing early in plugin lifecycle:
//!
//! ```rust
//! use zessionizer::observability::init_tracing;
//! use zessionizer::Config;
//!
//! let config = Config::default();
//! init_tracing(&config);
//!
//! tracing::debug!("plugin initialized");
//! ```
//!
//! # Modules
//!
//! - [`init`]: Tracing initialization and subscriber setup
//! - [`tracer`]: Custom OpenTelemetry tracer provider with file export
//! - [`span_formatter`]: OTLP JSON span serialization
//! - [`file_writer`]: Rotating file writer with size-based rotation

mod file_writer;
mod span_formatter;
mod tracer;
mod init;

pub use init::init_tracing;
