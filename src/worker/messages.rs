//! Worker thread message types for cross-thread communication.
//!
//! This module defines the request and response protocol between the main plugin
//! thread and the background worker thread that handles storage operations. It
//! also implements distributed tracing context propagation across thread boundaries.

use crate::domain::Project;
use serde::{Deserialize, Serialize};

/// Distributed tracing context for cross-thread span propagation.
///
/// Captures the current trace and span IDs from OpenTelemetry to maintain
/// trace continuity when passing messages to the worker thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    /// OpenTelemetry trace ID as a hex string.
    pub trace_id: String,

    /// Parent span ID for linking spans across threads.
    pub parent_span_id: String,
}

impl TraceContext {
    /// Creates a trace context from the current tracing span.
    ///
    /// Extracts the OpenTelemetry trace ID and span ID from the active span.
    /// Returns `None` if the current span context is invalid or not sampled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::worker::TraceContext;
    ///
    /// let context = TraceContext::from_current();
    /// if let Some(ctx) = context {
    ///     println!("Trace ID: {}", ctx.trace_id);
    /// }
    /// ```
    pub fn from_current() -> Option<Self> {
        use opentelemetry::trace::TraceContextExt;
        use tracing_opentelemetry::OpenTelemetrySpanExt;

        let span = tracing::Span::current();

        let otel_context = span.context();
        let span_ref = otel_context.span();
        let span_context = span_ref.span_context();

        if span_context.is_valid() {
            let trace_id_str = format!("{:032x}", span_context.trace_id());
            let parent_span_id_str = format!("{:016x}", span_context.span_id());

            tracing::debug!(
                trace_id = %trace_id_str,
                parent_span_id = %parent_span_id_str,
                "capturing trace context"
            );

            Some(Self {
                trace_id: trace_id_str,
                parent_span_id: parent_span_id_str,
            })
        } else {
            tracing::debug!("span context is not valid");
            None
        }
    }
}

/// Macro to generate builder methods for `WorkerMessage` variants.
///
/// Generates convenience constructors that automatically attach the current
/// trace context to each message variant.
macro_rules! worker_message_builders {
    (
        $(
            $builder_name:ident($variant:ident { $($field:ident: $ty:ty),* $(,)? })
        ),* $(,)?
    ) => {
        impl WorkerMessage {
            $(
                #[doc = concat!("Create a ", stringify!($variant), " message with current trace context")]
                pub fn $builder_name($($field: $ty),*) -> Self {
                    Self::$variant {
                        $($field,)*
                        trace_context: TraceContext::from_current(),
                    }
                }
            )*
        }
    };
}

worker_message_builders! {
    load_projects(LoadProjects { with_sessions: bool }),
    update_frecency(UpdateFrecency { path: String }),
    update_project_layout(UpdateProjectLayout { path: String, layout: Option<String> }),
    add_projects_batch(AddProjectsBatch { projects: Vec<(String, String)> }),
    sync_sessions(SyncSessions { active_sessions: Vec<String> }),
}

/// Messages sent from the main thread to the worker thread.
///
/// Each variant corresponds to a storage operation that should be performed
/// asynchronously. All variants include an optional trace context for distributed
/// tracing support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerMessage {
    /// Load all projects from storage, optionally including session information.
    LoadProjects {
        /// Whether to include active session associations.
        with_sessions: bool,

        /// Trace context for linking spans across threads.
        #[serde(skip_serializing_if = "Option::is_none")]
        trace_context: Option<TraceContext>,
    },

    /// Update the frecency data for a specific project.
    UpdateFrecency {
        /// Filesystem path of the project to update.
        path: String,

        /// Trace context for linking spans across threads.
        #[serde(skip_serializing_if = "Option::is_none")]
        trace_context: Option<TraceContext>,
    },

    /// Add or update multiple projects in a single transaction.
    AddProjectsBatch {
        /// Project tuples of (path, name) to add.
        projects: Vec<(String, String)>,

        /// Trace context for linking spans across threads.
        #[serde(skip_serializing_if = "Option::is_none")]
        trace_context: Option<TraceContext>,
    },

    /// Synchronize the sessions table with active Zellij sessions.
    SyncSessions {
        /// Names of currently active Zellij sessions.
        active_sessions: Vec<String>,

        /// Trace context for linking spans across threads.
        #[serde(skip_serializing_if = "Option::is_none")]
        trace_context: Option<TraceContext>,
    },

    /// Update the layout associated with a project.
    UpdateProjectLayout {
        /// Filesystem path of the project to update.
        path: String,

        /// New layout to associate with the project (None to clear).
        layout: Option<String>,

        /// Trace context for linking spans across threads.
        #[serde(skip_serializing_if = "Option::is_none")]
        trace_context: Option<TraceContext>,
    },
}

/// Responses sent from the worker thread back to the main thread.
///
/// Each variant corresponds to the completion of a worker operation, either
/// successfully with result data or with an error message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerResponse {
    /// Projects were successfully loaded from storage.
    ProjectsLoaded {
        /// The loaded projects, sorted by frecency.
        projects: Vec<Project>,
    },

    /// Project frecency was successfully updated.
    FrecencyUpdated {
        /// Path of the updated project.
        path: String,
    },

    /// Multiple projects were successfully added or updated.
    ProjectsBatchAdded {
        /// Number of projects in the batch.
        count: usize,

        /// All projects after the batch operation, sorted by frecency.
        projects: Vec<Project>,
    },

    /// Sessions were successfully synchronized.
    SessionsSynced {
        /// Number of sessions synchronized.
        count: usize,
    },

    /// An error occurred during the worker operation.
    Error {
        /// Human-readable error message.
        message: String,
    },

    /// Project layout was successfully updated.
    LayoutUpdated {
        /// Path of the project whose layout was updated.
        path: String,
    },
}
