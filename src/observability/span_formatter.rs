//! OTLP JSON span formatter.
//!
//! This module converts OpenTelemetry span data into OTLP (OpenTelemetry
//! Protocol) JSON format for file export. The output is compatible with OTLP
//! trace collectors and analysis tools.

use opentelemetry_sdk::export::trace::SpanData;
use opentelemetry_sdk::resource::Resource;
use serde_json::Value as JsonValue;

/// OTLP JSON span formatter.
///
/// Formats batches of spans into complete OTLP JSON documents with resource
/// attributes, scope information, and span details.
pub struct SpanFormatter {
    /// OpenTelemetry resource metadata (service name, etc.).
    resource: Resource,
}

impl SpanFormatter {
    /// Creates a new span formatter with resource metadata.
    ///
    /// # Parameters
    ///
    /// * `resource` - OpenTelemetry resource to include in formatted output
    pub const fn new(resource: Resource) -> Self {
        Self { resource }
    }

    /// Formats a batch of spans as an OTLP JSON document.
    ///
    /// Creates a complete OTLP JSON structure with:
    /// - `resourceSpans`: Array of resource-scope-span groupings
    /// - `resource.attributes`: Service name and metadata
    /// - `scopeSpans`: Array containing spans for the "Zessionizer" scope
    /// - `spans`: Array of span data
    ///
    /// # Parameters
    ///
    /// * `batch` - Vector of span data to format
    ///
    /// # Returns
    ///
    /// A `JsonValue` containing the complete OTLP document. This value can be
    /// serialized to a string with `.to_string()`.
    ///
    /// # OTLP Format
    ///
    /// ```json
    /// {
    ///   "resourceSpans": [{
    ///     "resource": {
    ///       "attributes": [{"key": "service.name", "value": {"stringValue": "Zessionizer"}}]
    ///     },
    ///     "scopeSpans": [{
    ///       "scope": {"name": "Zessionizer"},
    ///       "spans": [...]
    ///     }]
    ///   }]
    /// }
    /// ```
    pub fn format_batch(&self, batch: &[SpanData]) -> JsonValue {
        let resource_attrs: Vec<JsonValue> = self
            .resource
            .iter()
            .map(|(k, v)| {
                let value = Self::format_attribute_value(v);
                serde_json::json!({
                    "key": k.to_string(),
                    "value": value
                })
            })
            .collect();

        let spans_json: Vec<JsonValue> = batch
            .iter()
            .map(Self::format_span)
            .collect();

        serde_json::json!({
            "resourceSpans": [{
                "resource": {
                    "attributes": resource_attrs
                },
                "scopeSpans": [{
                    "scope": {
                        "name": "Zessionizer",
                    },
                    "spans": spans_json
                }]
            }]
        })
    }

    /// Formats a single span as OTLP JSON.
    ///
    /// Converts all span fields to OTLP format:
    /// - IDs as hex strings (trace ID: 32 chars, span ID: 16 chars)
    /// - Timestamps as nanoseconds since Unix epoch
    /// - Attributes, events, links as arrays
    /// - Status code as integer (0=unset, 1=ok, 2=error)
    ///
    /// # Parameters
    ///
    /// * `span` - Span data to format
    ///
    /// # Returns
    ///
    /// A `JsonValue` containing the OTLP span object.
    fn format_span(span: &SpanData) -> JsonValue {
        let kind = Self::span_kind_to_int(&span.span_kind);
        let attributes = Self::format_attributes(&span.attributes);
        let events = Self::format_events(&span.events);
        let links = Self::format_links(&span.links);
        let (status_code, status_message) = Self::format_status(&span.status);

        serde_json::json!({
            "traceId": format!("{:032x}", span.span_context.trace_id()),
            "spanId": format!("{:016x}", span.span_context.span_id()),
            "parentSpanId": if span.parent_span_id == opentelemetry::trace::SpanId::INVALID {
                String::new()
            } else {
                format!("{:016x}", span.parent_span_id)
            },
            "name": span.name,
            "kind": kind,
            "startTimeUnixNano": format!("{}", span.start_time.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or(std::time::Duration::from_secs(0)).as_nanos()),
            "endTimeUnixNano": format!("{}", span.end_time.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or(std::time::Duration::from_secs(0)).as_nanos()),
            "attributes": attributes,
            "events": events,
            "links": links,
            "status": {
                "code": status_code,
                "message": status_message,
            },
        })
    }

    /// Converts span kind to OTLP integer code.
    ///
    /// # Mapping
    ///
    /// - Internal: 1
    /// - Server: 2
    /// - Client: 3
    /// - Producer: 4
    /// - Consumer: 5
    const fn span_kind_to_int(kind: &opentelemetry::trace::SpanKind) -> u8 {
        match kind {
            opentelemetry::trace::SpanKind::Internal => 1,
            opentelemetry::trace::SpanKind::Server => 2,
            opentelemetry::trace::SpanKind::Client => 3,
            opentelemetry::trace::SpanKind::Producer => 4,
            opentelemetry::trace::SpanKind::Consumer => 5,
        }
    }

    /// Formats span attributes as OTLP JSON array.
    ///
    /// Each attribute is converted to `{"key": "...", "value": {...}}` format.
    fn format_attributes(attributes: &[opentelemetry::KeyValue]) -> Vec<JsonValue> {
        attributes
            .iter()
            .map(|kv| {
                let value = Self::format_attribute_value(&kv.value);
                serde_json::json!({
                    "key": kv.key.to_string(),
                    "value": value
                })
            })
            .collect()
    }

    /// Formats an attribute value as OTLP JSON.
    ///
    /// Maps OpenTelemetry value types to OTLP value types:
    /// - Bool → `{"boolValue": true}`
    /// - I64 → `{"intValue": "123"}` (as string)
    /// - F64 → `{"doubleValue": 1.23}`
    /// - String → `{"stringValue": "..."}`
    /// - Array → `{"stringValue": "[debug format]"}` (fallback)
    fn format_attribute_value(value: &opentelemetry::Value) -> JsonValue {
        use opentelemetry::Value;

        match value {
            Value::Bool(b) => serde_json::json!({ "boolValue": b }),
            Value::I64(i) => serde_json::json!({ "intValue": i.to_string() }),
            Value::F64(f) => serde_json::json!({ "doubleValue": f }),
            Value::String(s) => serde_json::json!({ "stringValue": s.to_string() }),
            Value::Array(_arr) => {
                serde_json::json!({ "stringValue": format!("{:?}", value) })
            }
        }
    }

    /// Formats span events as OTLP JSON array.
    ///
    /// Events include timestamp, name, and attributes.
    fn format_events(events: &[opentelemetry::trace::Event]) -> Vec<JsonValue> {
        events
            .iter()
            .map(|event| {
                let event_attrs = Self::format_attributes(&event.attributes);

                serde_json::json!({
                    "timeUnixNano": format!("{}", event.timestamp.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or(std::time::Duration::from_secs(0)).as_nanos()),
                    "name": event.name,
                    "attributes": event_attrs,
                })
            })
            .collect()
    }

    /// Formats span links as OTLP JSON array.
    ///
    /// Links include trace ID, span ID, and attributes.
    fn format_links(links: &[opentelemetry::trace::Link]) -> Vec<JsonValue> {
        links
            .iter()
            .map(|link| {
                let link_attrs = Self::format_attributes(&link.attributes);

                serde_json::json!({
                    "traceId": format!("{:032x}", link.span_context.trace_id()),
                    "spanId": format!("{:016x}", link.span_context.span_id()),
                    "attributes": link_attrs,
                })
            })
            .collect()
    }

    /// Formats span status as OTLP code and message.
    ///
    /// # Returns
    ///
    /// A tuple of `(code, message)`:
    /// - Unset: `(0, "")`
    /// - Ok: `(1, "")`
    /// - Error: `(2, "error description")`
    fn format_status(status: &opentelemetry::trace::Status) -> (u8, String) {
        match status {
            opentelemetry::trace::Status::Unset => (0, String::new()),
            opentelemetry::trace::Status::Ok => (1, String::new()),
            opentelemetry::trace::Status::Error { description } => (2, description.to_string()),
        }
    }
}

impl std::fmt::Debug for SpanFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpanFormatter").finish()
    }
}
