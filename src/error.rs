//! Error types for the observability library.

use opentelemetry_otlp::ExporterBuildError;
use thiserror::Error;

/// Main error type for the otel-rs library.
#[derive(Debug, Error)]
pub enum OtelError {
    /// Error building an OTLP exporter.
    #[error("OTLP exporter build error: {0}")]
    ExporterBuildError(#[from] ExporterBuildError),

    /// Unknown or unsupported OTLP protocol.
    #[error("unknown OTLP protocol: {0} (supported: 'grpc', 'http')")]
    UnknownProtocol(String),

    /// Metrics shutdown error.
    #[error("metrics shutdown error: {0}")]
    MetricsShutdown(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// Initialization error.
    #[error("initialization error: {0}")]
    Initialization(String),

    /// Tracing subscriber already set globally.
    #[error("tracing subscriber already initialized — only one can be set globally")]
    SubscriberAlreadySet,
}

impl OtelError {
    /// Create a configuration error.
    #[must_use]
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create an initialization error.
    #[must_use]
    pub fn init(msg: impl Into<String>) -> Self {
        Self::Initialization(msg.into())
    }
}

/// Result type alias for otel-rs operations.
pub type OtelResult<T> = Result<T, OtelError>;

/// Extension trait for adding context to errors.
pub trait ErrorContext<T> {
    /// Wrap the error with additional context.
    fn context(self, ctx: impl Into<String>) -> OtelResult<T>;

    /// Wrap the error with lazily-evaluated context.
    fn with_context<F: FnOnce() -> String>(self, f: F) -> OtelResult<T>;
}

impl<T, E: std::error::Error + 'static> ErrorContext<T> for Result<T, E> {
    fn context(self, ctx: impl Into<String>) -> OtelResult<T> {
        self.map_err(|e| OtelError::Initialization(format!("{}: {e}", ctx.into())))
    }

    fn with_context<F: FnOnce() -> String>(self, f: F) -> OtelResult<T> {
        self.map_err(|e| OtelError::Initialization(format!("{}: {e}", f())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = OtelError::UnknownProtocol("websocket".into());
        assert!(err.to_string().contains("websocket"));
    }

    #[test]
    fn config_error() {
        let err = OtelError::config("bad endpoint");
        assert!(err.to_string().contains("bad endpoint"));
    }
}
