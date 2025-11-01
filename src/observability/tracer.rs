//! Custom OpenTelemetry tracer provider with file-based span export.
//!
//! This module implements a custom `SpanExporter` that writes spans to a
//! rotating JSON file instead of sending them over the network. This enables
//! offline trace analysis and debugging in sandbox environments.

use super::file_writer::FileWriter;
use super::span_formatter::SpanFormatter;
use futures_util::future::BoxFuture;
use opentelemetry::trace::TraceError;
use opentelemetry_sdk::export::trace::{ExportResult, SpanData, SpanExporter};
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::trace::TracerProvider;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// File-based OpenTelemetry span exporter.
///
/// Implements the `SpanExporter` trait to write spans to a rotating file in
/// OTLP JSON format. Spans are formatted into complete OTLP batches with
/// resource attributes and scope information.
struct FileSpanExporter {
    /// File writer with rotation support.
    writer: FileWriter,
    /// OTLP JSON formatter.
    formatter: SpanFormatter,
    /// Shutdown flag (prevents export after shutdown).
    is_shutdown: AtomicBool,
}

impl FileSpanExporter {
    /// Creates a new file-based span exporter.
    ///
    /// # Parameters
    ///
    /// * `file_path` - Path to the JSON trace file
    /// * `resource` - OpenTelemetry resource metadata (service name, etc.)
    const fn new(file_path: PathBuf, resource: Resource) -> Self {
        Self {
            writer: FileWriter::new(file_path),
            formatter: SpanFormatter::new(resource),
            is_shutdown: AtomicBool::new(false),
        }
    }
}

impl SpanExporter for FileSpanExporter {
    /// Exports a batch of spans to the file.
    ///
    /// Formats the batch as OTLP JSON and writes it as a single line to the
    /// file. Each line is a complete OTLP JSON document with `resourceSpans`,
    /// `scopeSpans`, and `spans` arrays.
    ///
    /// # Parameters
    ///
    /// * `batch` - Batch of span data to export
    ///
    /// # Returns
    ///
    /// - `Ok(())` if spans were written successfully
    /// - `Err(TraceError)` if the exporter is shut down or write fails
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, ExportResult> {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Box::pin(std::future::ready(Err(TraceError::from(
                "exporter is shut down",
            ))));
        }

        let json = self.formatter.format_batch(&batch);
        let json_string = json.to_string();

        match self.writer.write_line(&json_string) {
            Ok(()) => Box::pin(std::future::ready(Ok(()))),
            Err(e) => {
                Box::pin(std::future::ready(Err(TraceError::from(e.to_string()))))
            }
        }
    }

    /// Shuts down the exporter.
    ///
    /// Sets the shutdown flag to prevent further exports. Does not flush or
    /// close the file (handled by Drop).
    fn shutdown(&mut self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
    }

    /// Updates the resource metadata.
    ///
    /// No-op implementation (resource is set during construction).
    ///
    /// # Parameters
    ///
    /// * `res` - New resource metadata (ignored)
    fn set_resource(&mut self, res: &Resource) {
        let _ = res;
    }
}

impl std::fmt::Debug for FileSpanExporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSpanExporter")
            .field("writer", &self.writer)
            .field("formatter", &self.formatter)
            .field("is_shutdown", &self.is_shutdown)
            .finish()
    }
}

/// Creates a tracer provider with file-based export.
///
/// Constructs a complete OpenTelemetry tracer provider configured with:
/// - Custom file-based span exporter
/// - Resource metadata (service name, etc.)
/// - Simple export strategy (immediate, non-batched)
///
/// # Parameters
///
/// * `file_path` - Path to the JSON trace file
/// * `resource` - OpenTelemetry resource metadata
///
/// # Returns
///
/// A configured `TracerProvider` ready for use with `tracing-opentelemetry`.
///
/// # Example
///
/// ```rust
/// use opentelemetry_sdk::resource::Resource;
/// use opentelemetry::KeyValue;
/// use std::path::PathBuf;
///
/// let resource = Resource::new(vec![KeyValue::new("service.name", "myapp")]);
/// let path = PathBuf::from("/tmp/traces.json");
/// let provider = create_tracer_provider(path, resource);
/// ```
pub fn create_tracer_provider(
    file_path: PathBuf,
    resource: Resource,
) -> TracerProvider {
    let exporter = FileSpanExporter::new(file_path, resource.clone());

    TracerProvider::builder()
        .with_config(
            opentelemetry_sdk::trace::Config::default()
                .with_resource(resource)
        )
        .with_simple_exporter(exporter)
        .build()
}
