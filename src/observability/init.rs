//! Tracing initialization and subscriber setup.
//!
//! This module configures the tracing subscriber with OpenTelemetry integration,
//! setting up the complete observability pipeline from `tracing` macros to file
//! export.

use super::tracer;
use crate::Config;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::resource::Resource;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes the tracing subscriber with file-based OTLP export.
///
/// Sets up a tracing subscriber pipeline that:
/// 1. Filters spans based on configured trace level
/// 2. Exports spans to OpenTelemetry
/// 3. Serializes spans to OTLP JSON format
/// 4. Writes to rotating file with backups
///
/// # Parameters
///
/// * `config` - Plugin configuration containing `trace_level` option
///
/// # Trace Level Resolution
///
/// Level is determined by:
/// 1. `config.trace_level` if set
/// 2. Default: `"info"`
///
/// # File Location
///
/// Traces are written to: `~/.local/share/zellij/zessionizer/zessionizer-otlp.json`
///
/// The plugin uses `/data/zessionizer-otlp.json` in Zellij's sandbox environment,
/// which typically maps to the path above when Zellij is started from the user's
/// home directory.
///
/// # Initialization Behavior
///
/// - Creates data directory if it doesn't exist
/// - Silently fails if directory creation fails (observability is optional)
/// - Idempotent: Safe to call multiple times (only first call takes effect)
///
/// # Example
///
/// ```rust
/// use zessionizer::observability::init_tracing;
/// use zessionizer::Config;
///
/// let config = Config {
///     trace_level: Some("debug".to_string()),
///     ..Default::default()
/// };
///
/// init_tracing(&config);
///
/// tracing::debug!("tracing is now active");
/// ```
pub fn init_tracing(config: &Config) {
    let level = config
        .trace_level
        .clone()
        .unwrap_or_else(|| "info".to_string());

    let data_dir = crate::infrastructure::paths::get_data_dir();
    if let Err(_e) = std::fs::create_dir_all(&data_dir) {
        // Silently fail if we can't create the directory
        return;
    }

    let resource = Resource::new(vec![opentelemetry::KeyValue::new(
        "service.name",
        "Zessionizer",
    )]);

    let trace_file = data_dir.join("zessionizer-otlp.json");
    let provider = tracer::create_tracer_provider(trace_file, resource);

    let tracer = provider.tracer("Zessionizer");
    let otel_layer = OpenTelemetryLayer::new(tracer);

    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new(level))
        .with(otel_layer);

    let _ = subscriber.try_init();
}
