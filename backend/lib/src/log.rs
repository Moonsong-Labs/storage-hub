//! Logging configuration and setup for the StorageHub MSP backend
//!
//! This module provides custom logging infrastructure:
//! - The choice between JSON logging using Bunyan format or human-readable text logging
//! - Auto-detection based on whether the output is a TTY (JSON if non-TTY, Text if TTY)
//! - A custom writer that replaces "log." prefix with "backend_log." in Bunyan logs
//! to avoid conflicts with reserved fields in log ingestion tools

use std::io::Write;

use axum::{extract::MatchedPath, http::Request};
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{
    fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use crate::config::LogFormat;

/// Custom writer that post-processes Bunyan JSON logs by:
/// 1. Replacing "log." prefix with "backend_log." to avoid conflicts with log ingestion tools
/// 2. Expanding path_params from a single string into individual fields
struct LogTransformWriter<W: Write> {
    inner: W,
}

impl<W: Write> LogTransformWriter<W> {
    fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write> Write for LogTransformWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Convert buffer to string, apply transformations, and write
        if let Ok(s) = std::str::from_utf8(buf) {
            // First, replace the log.* prefix
            let mut modified = s.replace("\"log.", "\"backend_log.");

            // Then, expand path_params from a single string to individual fields
            modified = expand_path_params(&modified);

            self.inner.write_all(modified.as_bytes())?;
            Ok(buf.len())
        } else {
            // If not valid UTF-8, write as-is
            self.inner.write(buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Wrapper to make LogTransformWriter implement MakeWriter
struct LogTransformMakeWriter;

impl<'a> MakeWriter<'a> for LogTransformMakeWriter {
    type Writer = LogTransformWriter<std::io::Stdout>;

    fn make_writer(&'a self) -> Self::Writer {
        LogTransformWriter::new(std::io::stdout())
    }
}

/// Expands the path_params field from a single string into individual fields
///
/// Transforms: `"path_params":"bucket_id=my-bucket, file_key=document.pdf"`
/// Into: `"path_params.bucket_id":"my-bucket","path_params.file_key":"document.pdf"`
///
/// This allows log aggregation systems to index path parameters by name, making it easier to filter logs by path parameter.
fn expand_path_params(json_str: &str) -> String {
    // Look for the path_params field in the JSON
    if let Some(start_idx) = json_str.find("\"path_params\":\"") {
        // Find the closing quote for the path_params value
        let value_start = start_idx + "\"path_params\":\"".len();
        if let Some(end_idx) = json_str[value_start..].find('\"') {
            let params_str = &json_str[value_start..value_start + end_idx];

            // If empty, just remove the path_params field entirely
            if params_str.is_empty() {
                // Remove the path_params field and any trailing comma
                let before = &json_str[..start_idx];
                let after = &json_str[value_start + end_idx + 1..];

                // Handle comma removal
                let cleaned_before = if before.trim_end().ends_with(',') {
                    before.trim_end().trim_end_matches(',')
                } else {
                    before
                };

                let cleaned_after = if after.trim_start().starts_with(',') {
                    &after[1..]
                } else {
                    after
                };

                return format!("{}{}", cleaned_before, cleaned_after);
            }

            // Parse the params_str and create individual fields
            let individual_fields: Vec<String> = params_str
                .split(", ")
                .filter_map(|param| {
                    let parts: Vec<&str> = param.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        Some(format!("\"path_params.{}\":\"{}\"", parts[0], parts[1]))
                    } else {
                        None
                    }
                })
                .collect();

            if individual_fields.is_empty() {
                return json_str.to_string();
            }

            // Replace the original path_params field with individual fields
            let before = &json_str[..start_idx];
            let after = &json_str[value_start + end_idx + 1..];
            let expanded = individual_fields.join(",");

            return format!("{}{}{}", before, expanded, after);
        }
    }

    json_str.to_string()
}

/// Initialize logging with the specified format
///
/// This function sets up the tracing subscriber with either JSON (Bunyan) or
/// text format logging based on the provided configuration.
///
/// For JSON format, a custom writer is used that replaces the "log." prefix
/// with "backend_log." to avoid conflicts with log ingestion tool reserved fields
/// while preserving all debugging information.
pub fn initialize_logging(log_format: LogFormat) {
    let env_filter = EnvFilter::from_default_env();

    // Resolve the actual format to use
    let format = log_format.resolve();

    match format {
        LogFormat::Json => {
            // Machine-readable JSON logging using Bunyan format
            tracing_subscriber::registry()
                .with(env_filter)
                .with(JsonStorageLayer)
                .with(BunyanFormattingLayer::new(
                    "storage-hub-backend".to_string(),
                    LogTransformMakeWriter,
                ))
                .init();
        }
        LogFormat::Text => {
            // Human-readable text logging
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .init();
        }
        LogFormat::Auto => {
            // This should have been resolved, but handle it just in case
            let resolved = log_format.resolve();
            initialize_logging(resolved);
        }
    }
}

/// Creates a tracing layer for HTTP requests that attaches endpoint information
/// to all logs within the request span.
///
/// This layer adds the following fields to all logs within an HTTP request:
/// - `endpoint`: The matched route pattern (e.g., "/buckets/{bucket_id}/files")
/// - `method`: The HTTP method (GET, POST, etc.)
/// - `path_params.{name}`: Individual path parameters (e.g., `path_params.bucket_id: "123"`)
pub fn create_http_trace_layer<B>() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    impl tower_http::trace::MakeSpan<B> + Clone,
> {
    TraceLayer::new_for_http().make_span_with(|request: &Request<B>| {
        // Extract the matched path pattern
        let matched_path = request
            .extensions()
            .get::<MatchedPath>()
            .map(|path| path.as_str())
            .unwrap_or("unknown");

        // Get the actual URI path
        let uri_path = request.uri().path();

        // Extract path parameters by comparing matched path to actual URI
        let path_params = extract_path_params(matched_path, uri_path);

        // Format path parameters as a string that will be expanded into individual fields
        // Format: "key1=value1, key2=value2"
        let params_str = if path_params.is_empty() {
            String::new()
        } else {
            path_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        };

        // Create a span with endpoint, method, and path parameters
        // The path_params string will be expanded into individual fields by the writer
        info_span!(
            "http_request",
            method = %request.method(),
            endpoint = %matched_path,
            path_params = %params_str,
        )
    })
}

/// Extracts path parameters by comparing the matched route pattern to the actual URI path
///
/// Example:
/// - Matched: "/buckets/{bucket_id}/files/{file_key}"
/// - URI: "/buckets/my-bucket/files/document.pdf"
/// - Returns: [("bucket_id", "my-bucket"), ("file_key", "document.pdf")]
fn extract_path_params(matched_path: &str, uri_path: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();

    let matched_segments: Vec<&str> = matched_path.split('/').collect();
    let uri_segments: Vec<&str> = uri_path.split('/').collect();

    // Only extract if both paths have the same number of segments
    if matched_segments.len() == uri_segments.len() {
        for (matched_seg, uri_seg) in matched_segments.iter().zip(uri_segments.iter()) {
            // Check if this is a path parameter (e.g., "{bucket_id}")
            if matched_seg.starts_with('{') && matched_seg.ends_with('}') {
                // Extract parameter name without braces
                let param_name = matched_seg
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .to_string();
                params.push((param_name, uri_seg.to_string()));
            }
        }
    }

    params
}
