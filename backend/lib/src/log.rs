//! Logging configuration and setup for the StorageHub MSP backend
//!
//! This module provides custom logging infrastructure:
//! - The choice between JSON logging using Bunyan format or human-readable text logging
//! - Auto-detection based on whether the output is a TTY (JSON if non-TTY, Text if TTY)
//! - A custom writer that replaces "log." prefix with "backend_log." in Bunyan logs
//! to avoid conflicts with reserved fields in log ingestion tools

use std::io::Write;

use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{
    fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use crate::config::LogFormat;

/// Custom writer that replaces "log." prefix with "backend_log." in Bunyan logs
/// to avoid conflicts with reserved fields in log ingestion tools
struct PrefixReplacingWriter<W: Write> {
    inner: W,
}

impl<W: Write> PrefixReplacingWriter<W> {
    fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write> Write for PrefixReplacingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Convert buffer to string, replace the prefix, and write
        if let Ok(s) = std::str::from_utf8(buf) {
            let replaced = s.replace("\"log.", "\"backend_log.");
            self.inner.write_all(replaced.as_bytes())?;
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

/// Wrapper to make PrefixReplacingWriter implement MakeWriter
struct PrefixReplacingMakeWriter;

impl<'a> MakeWriter<'a> for PrefixReplacingMakeWriter {
    type Writer = PrefixReplacingWriter<std::io::Stdout>;

    fn make_writer(&'a self) -> Self::Writer {
        PrefixReplacingWriter::new(std::io::stdout())
    }
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
                    PrefixReplacingMakeWriter,
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
