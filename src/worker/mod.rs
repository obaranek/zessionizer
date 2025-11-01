//! Background worker thread for asynchronous storage operations.
//!
//! This module implements the worker thread that handles all storage I/O to avoid
//! blocking the main plugin UI thread. It uses Zellij's worker API for cross-thread
//! communication and includes distributed tracing support for observability.
//!
//! # Architecture
//!
//! - `messages`: Request/response protocol types with trace context propagation
//! - `handler`: Worker implementation and message processing logic

pub mod handler;
pub mod messages;

pub use handler::ZessionizerWorker;
pub use messages::{TraceContext, WorkerMessage, WorkerResponse};
