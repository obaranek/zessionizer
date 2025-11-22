//! Worker thread implementation for asynchronous storage operations.
//!
//! This module implements the Zellij worker thread interface, handling all storage
//! operations asynchronously to avoid blocking the main plugin rendering loop. It
//! includes distributed tracing support for cross-thread observability.

use crate::domain::error::{Result, ZessionizerError};
use crate::domain::Project;
use crate::infrastructure::paths;
use crate::storage::backend::Storage;
use crate::storage::models::ProjectRecord;
use crate::storage::{sort_by_frecency, JsonStorage};
use crate::worker::{WorkerMessage, WorkerResponse};
use serde::{Deserialize, Serialize};
use zellij_tile::prelude::{PluginMessage, ZellijWorker};
use zellij_tile::shim::post_message_to_plugin;

/// Worker thread state for handling storage operations.
///
/// This struct runs on a separate thread spawned by Zellij and processes
/// messages sent from the main plugin thread. The storage backend is
/// initialized lazily on first message receipt.
#[derive(Serialize, Deserialize, Default)]
pub struct ZessionizerWorker {
    /// Storage backend, initialized lazily on first use.
    #[serde(skip)]
    storage: Option<Box<dyn Storage>>,
}

impl ZessionizerWorker {
    /// Creates a new worker with an initialized storage backend.
    ///
    /// Uses JSON file storage for persisting project and session data.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage backend cannot be initialized.
    pub fn new(_backend_param: String) -> Result<Self> {
        let path = paths::get_data_dir().join("projects.json");
        let storage: Box<dyn Storage> = Box::new(JsonStorage::new(path)?);
        Ok(Self { storage: Some(storage) })
    }

    /// Returns a mutable reference to the storage backend, failing if not initialized.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage has not been initialized yet.
    fn get_storage(&mut self) -> Result<&mut Box<dyn Storage>> {
        self.storage
            .as_mut()
            .ok_or_else(|| ZessionizerError::Worker("Storage not initialized".to_string()))
    }

    /// Converts a storage-layer `ProjectRecord` to a domain `Project`.
    ///
    /// This transformation is necessary because the worker returns domain types
    /// to the main thread, not storage types.
    fn project_record_to_project(record: ProjectRecord) -> Project {
        Project {
            id: None,
            path: record.path,
            name: record.name,
            last_accessed: record.last_accessed.unwrap_or(record.created_at),
            created_at: record.created_at,
            layout: record.layout,
        }
    }

    /// Helper for handling storage operation results with consistent logging.
    ///
    /// This function standardizes error handling and success logging across all
    /// storage operations in the worker.
    fn handle_db_result<T, F>(operation: &str, result: Result<T>, on_success: F) -> WorkerResponse
    where
        F: FnOnce(T) -> WorkerResponse,
    {
        match result {
            Ok(value) => {
                tracing::debug!(operation = operation, "storage operation successful");
                on_success(value)
            }
            Err(e) => {
                tracing::debug!(operation = operation, error = %e, "storage operation failed");
                WorkerResponse::Error {
                    message: format!("{operation}: {e}"),
                }
            }
        }
    }

    /// Handles the `LoadProjects` message.
    ///
    /// Retrieves all projects from storage, sorted by frecency.
    fn handle_load_projects(&mut self, _with_sessions: bool) -> WorkerResponse {
        Self::handle_db_result(
            "load projects",
            self.get_storage().and_then(|storage| storage.get_all_projects()),
            |mut records| {
                sort_by_frecency(&mut records);

                tracing::debug!(
                    project_count = records.len(),
                    "projects loaded from storage (sorted by frecency)"
                );
                let projects = records
                    .into_iter()
                    .map(Self::project_record_to_project)
                    .collect();
                WorkerResponse::ProjectsLoaded { projects }
            },
        )
    }

    /// Handles the `UpdateFrecency` message.
    ///
    /// Updates the last accessed time and access count for a project.
    fn handle_update_frecency(&mut self, path: String) -> WorkerResponse {
        let timestamp = chrono::Utc::now().timestamp();

        Self::handle_db_result(
            "update frecency",
            self.get_storage()
                .and_then(|storage| storage.update_project_access(&path, timestamp)),
            |()| {
                tracing::debug!(project_path = %path, timestamp = timestamp, "frecency updated");
                WorkerResponse::FrecencyUpdated { path }
            },
        )
    }

    /// Handles the `AddProjectsBatch` message.
    ///
    /// Adds or updates multiple projects in a single transaction, then returns
    /// all projects sorted by frecency.
    fn handle_add_projects_batch(&mut self, projects: Vec<(String, String)>) -> WorkerResponse {
        let now = chrono::Utc::now().timestamp();
        let records: Vec<ProjectRecord> = projects
            .into_iter()
            .map(|(path, name)| ProjectRecord {
                path,
                name,
                last_accessed: Some(now),
                created_at: now,
                access_count: 1,
                layout: None,
            })
            .collect();

        let count = records.len();

        Self::handle_db_result(
            "add projects batch",
            self.get_storage().and_then(|storage| storage.add_projects_batch(&records)),
            |mut project_records| {
                sort_by_frecency(&mut project_records);

                tracing::debug!(project_count = count, "projects batch added to storage");
                let projects = project_records
                    .into_iter()
                    .map(Self::project_record_to_project)
                    .collect();
                WorkerResponse::ProjectsBatchAdded { count, projects }
            },
        )
    }

    /// Handles the `SyncSessions` message.
    ///
    /// Synchronizes the sessions table with the list of active Zellij sessions.
    fn handle_sync_sessions(&mut self, active_sessions: &[String]) -> WorkerResponse {
        let count = active_sessions.len();

        Self::handle_db_result(
            "sync sessions",
            self.get_storage().and_then(|storage| storage.sync_sessions(active_sessions)),
            |()| {
                tracing::debug!(session_count = count, "sessions synced successfully");
                WorkerResponse::SessionsSynced { count }
            },
        )
    }

    /// Handles the `UpdateProjectLayout` message.
    ///
    /// Updates the layout associated with a specific project.
    fn handle_update_project_layout(&mut self, path: String, layout: Option<String>) -> WorkerResponse {
        Self::handle_db_result(
            "update project layout",
            self.get_storage().and_then(|storage| storage.update_project_layout(&path, layout)),
            |_| {
                tracing::debug!(project_path = %path, "project layout updated");
                WorkerResponse::LayoutUpdated { path }
            },
        )
    }

    /// Attaches the parent trace context from a message to the current thread.
    ///
    /// This function reconstructs the OpenTelemetry context from the serialized
    /// trace information in the message, allowing spans created in the worker
    /// thread to be linked to their parent spans in the main thread.
    ///
    /// Returns a context guard that must be held for the duration of the operation.
    fn attach_parent_trace_context(message: &WorkerMessage) -> Option<opentelemetry::ContextGuard> {
        use opentelemetry::trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState};

        let trace_context = match message {
            WorkerMessage::LoadProjects { trace_context, .. }
            | WorkerMessage::UpdateFrecency { trace_context, .. }
            | WorkerMessage::UpdateProjectLayout { trace_context, .. }
            | WorkerMessage::AddProjectsBatch { trace_context, .. }
            | WorkerMessage::SyncSessions { trace_context, .. } => trace_context,
        }
        .as_ref()?;

        let trace_id = TraceId::from_hex(&trace_context.trace_id).ok()?;
        let span_id = SpanId::from_hex(&trace_context.parent_span_id).ok()?;

        let span_context = SpanContext::new(
            trace_id,
            span_id,
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        );

        let otel_context = opentelemetry::Context::current().with_remote_span_context(span_context);

        Some(otel_context.attach())
    }

    /// Processes a worker message and returns the appropriate response.
    ///
    /// This is the main message handling entry point, dispatching to specific
    /// handlers based on the message variant. Automatically attaches trace context
    /// and creates a tracing span for the operation.
    pub fn handle_message(&mut self, message: WorkerMessage) -> WorkerResponse {
        let _context_guard = Self::attach_parent_trace_context(&message);

        let span = tracing::debug_span!("worker_handle_message", message_type = ?message);
        let _guard = span.entered();

        match message {
            WorkerMessage::LoadProjects { with_sessions, .. } => {
                self.handle_load_projects(with_sessions)
            }

            WorkerMessage::UpdateFrecency { path, .. } => self.handle_update_frecency(path),

            WorkerMessage::UpdateProjectLayout { path, layout, .. } => self.handle_update_project_layout(path, layout),

            WorkerMessage::AddProjectsBatch { projects, .. } => {
                self.handle_add_projects_batch(projects)
            }

            WorkerMessage::SyncSessions { active_sessions, .. } => {
                self.handle_sync_sessions(&active_sessions)
            }
        }
    }
}

/// Initializes tracing for the worker thread.
///
/// Sets up the same tracing configuration as the main thread, ensuring logs
/// from both threads are written to the same file.
fn init_worker_tracing() {
    use crate::observability;
    use crate::Config;

    let config = Config::default();
    observability::init_tracing(&config);
}

/// Tracks whether worker tracing has been initialized.
///
/// Used to ensure tracing is only set up once per worker thread lifetime.
static WORKER_TRACING_INITIALIZED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

impl ZellijWorker<'_> for ZessionizerWorker {
    /// Handles incoming messages from the main plugin thread.
    ///
    /// This is the Zellij worker interface entry point. It:
    /// 1. Initializes tracing on first message (once per worker lifetime)
    /// 2. Lazy-initializes the storage backend if needed
    /// 3. Deserializes the message payload
    /// 4. Processes the message via `handle_message`
    /// 5. Serializes and sends the response back to the main thread
    ///
    /// # Arguments
    ///
    /// * `message` - Message name used for routing the response
    /// * `payload` - JSON-serialized `WorkerMessage`
    fn on_message(&mut self, message: String, payload: String) {
        if !WORKER_TRACING_INITIALIZED.load(std::sync::atomic::Ordering::Relaxed) {
            init_worker_tracing();
            WORKER_TRACING_INITIALIZED.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        if self.storage.is_none() {
            match Self::new(String::new()) {
                Ok(worker) => {
                    self.storage = worker.storage;
                }
                Err(e) => {
                    tracing::debug!(error = %e, "failed to initialize storage");
                    let error_response = WorkerResponse::Error {
                        message: format!("Failed to initialize storage: {e}"),
                    };
                    if let Ok(payload) = serde_json::to_string(&error_response) {
                        post_message_to_plugin(PluginMessage {
                            name: message,
                            payload,
                            worker_name: None,
                        });
                    }
                    return;
                }
            }
        }

        let worker_message: WorkerMessage = match serde_json::from_str(&payload) {
            Ok(msg) => msg,
            Err(e) => {
                tracing::debug!(error = %e, "failed to deserialize worker message");
                return;
            }
        };

        let response = self.handle_message(worker_message);

        match serde_json::to_string(&response) {
            Ok(payload) => {
                let plugin_message = PluginMessage {
                    name: message,
                    payload,
                    worker_name: None,
                };
                post_message_to_plugin(plugin_message);
            }
            Err(e) => {
                tracing::debug!(error = %e, "failed to serialize worker response");
            }
        }
    }
}
