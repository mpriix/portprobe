//! Logging configuration for PortProbe.
//!
//! Initializes `tracing-subscriber` with:
//! - Local timezone timestamps (RFC 3339 format).
//! - Environment-based log level filtering via `RUST_LOG`.
//! - Compact text or structured JSON output format suitable for terminal viewing or SIEM ingestion.
//!
//! # Important: Single-threaded initialization
//!
//! This module's `init_logger` function must be called *before* the
//! Tokio runtime is started. The `time` crate's
//! `UtcOffset::current_local_offset()` uses `localtime_r` on Unix,
//! which reads the `TZ` environment variable. In a multi-threaded
//! process, this can race with `setenv` calls, leading to undefined
//! behavior.
//!
//! By calling `init_logger()` in a single-threaded context (before
//! `tokio::runtime::Runtime::new()`), the timezone offset is safely
//! captured once and reused for all subsequent log entries.
//!
//! If the local offset cannot be determined (e.g., on some Linux
//! configurations), timestamps will silently fall back to UTC.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use tracing_subscriber::{fmt, EnvFilter};

/// Format of emitted log records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable compact single-line format for terminal output.
    #[default]
    Text,
    /// Structured JSON Lines format for SIEM ingestion (Splunk, Elastic, QRadar).
    Json,
}

/// Initialize the global tracing subscriber with the chosen output format.
///
/// # Log level configuration
///
/// Set the `RUST_LOG` environment variable to control log verbosity:
/// - `RUST_LOG=error` — Errors only
/// - `RUST_LOG=warn`  — Errors and warnings
/// - `RUST_LOG=info`  — Default; connection lifecycle events
/// - `RUST_LOG=debug` — Detailed diagnostics (peer disconnects, etc.)
/// - `RUST_LOG=trace` — Maximum verbosity
pub fn init_logger(format: LogFormat) {
    // Attempt to read RUST_LOG; default to "info" if unset or invalid.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Capture the local timezone offset while still single-threaded.
    // Falls back to UTC if the offset cannot be determined.
    let local_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let timer = fmt::time::OffsetTime::new(local_offset, Rfc3339);

    match format {
        LogFormat::Text => {
            fmt()
                .with_env_filter(filter)
                .with_target(false)
                .with_timer(timer)
                .compact()
                .init();
        }
        LogFormat::Json => {
            fmt()
                .with_env_filter(filter)
                .with_timer(timer)
                .json()
                .flatten_event(true)
                .init();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_format_default() {
        assert_eq!(LogFormat::default(), LogFormat::Text);
    }

    #[test]
    fn test_log_format_serde() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Config {
            format: LogFormat,
        }
        let cfg = Config {
            format: LogFormat::Text,
        };
        let toml_str = toml::to_string(&cfg).unwrap();
        assert!(toml_str.contains("format = \"text\""));
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed, cfg);

        let cfg_json = Config {
            format: LogFormat::Json,
        };
        let toml_json = toml::to_string(&cfg_json).unwrap();
        assert!(toml_json.contains("format = \"json\""));
        let parsed_json: Config = toml::from_str(&toml_json).unwrap();
        assert_eq!(parsed_json, cfg_json);
    }

    #[test]
    fn test_log_format_variants() {
        assert_ne!(LogFormat::Text, LogFormat::Json);
    }
}
