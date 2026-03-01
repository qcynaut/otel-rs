//! Span extensions for error recording and status management.
//!
//! Provides ergonomic helpers for recording errors, exceptions, and
//! managing span status following OpenTelemetry semantic conventions.

use std::fmt::Display;

use opentelemetry::trace::{Status, TraceContextExt};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Extension trait for enhanced span operations.
pub trait SpanExt {
    /// Record an error on the span and set status to Error.
    fn record_error<E: Display>(&self, error: &E);

    /// Record a structured exception following OTel semantic conventions.
    fn record_exception(&self, error_type: &str, message: &str, stacktrace: Option<&str>);

    /// Record a Result — if Err, records the error to the span.
    fn record_result<T, E: Display>(&self, result: &Result<T, E>) -> &Self;

    /// Set span status to OK.
    fn set_ok(&self);

    /// Set span status to Error with a message.
    fn set_error(&self, message: &str);

    /// Set a string attribute on the span.
    fn set_string_attribute(&self, key: &'static str, value: String);

    /// Set an i64 attribute on the span.
    fn set_i64_attribute(&self, key: &'static str, value: i64);
}

impl SpanExt for Span {
    fn record_error<E: Display>(&self, error: &E) {
        let msg = error.to_string();
        let ty = std::any::type_name::<E>();

        self.set_error(&msg);
        self.record("exception.message", msg.as_str());
        self.record("exception.type", ty);
        self.record("otel.status_code", "ERROR");

        tracing::error!(
            parent: self,
            error.message = %msg,
            error.type_name = %ty,
            "Exception recorded on span"
        );
    }

    fn record_exception(&self, error_type: &str, message: &str, stacktrace: Option<&str>) {
        self.set_error(message);
        self.record("exception.type", error_type);
        self.record("exception.message", message);
        if let Some(st) = stacktrace {
            self.record("exception.stacktrace", st);
        }
        self.record("otel.status_code", "ERROR");
    }

    fn record_result<T, E: Display>(&self, result: &Result<T, E>) -> &Self {
        if let Err(e) = result {
            self.record_error(e);
        }
        self
    }

    fn set_ok(&self) {
        self.record("otel.status_code", "OK");
    }

    fn set_error(&self, message: &str) {
        let context = self.context();
        let otel_span = context.span();
        otel_span.set_status(Status::error(message.to_string()));
    }

    fn set_string_attribute(&self, key: &'static str, value: String) {
        self.record(key, value.as_str());
    }

    fn set_i64_attribute(&self, key: &'static str, value: i64) {
        self.record(key, value);
    }
}

/// Extension trait for recording Result errors to the current span.
///
/// # Example
///
/// ```rust,ignore
/// use otel_rs::InstrumentedResult;
///
/// async fn my_fn() -> Result<String, MyError> {
///     fallible_call().await.record_to_span()
/// }
/// ```
pub trait InstrumentedResult<T, E> {
    /// Record any error to the current span, returning the result unchanged.
    fn record_to_span(self) -> Result<T, E>;

    /// Record any error to a specific span, returning the result unchanged.
    fn record_to(self, span: &Span) -> Result<T, E>;
}

impl<T, E: Display> InstrumentedResult<T, E> for Result<T, E> {
    fn record_to_span(self) -> Self {
        if let Err(ref e) = self {
            Span::current().record_error(e);
        }
        self
    }

    fn record_to(self, span: &Span) -> Self {
        if let Err(ref e) = self {
            span.record_error(e);
        }
        self
    }
}

/// Context for tracking operation timing and recording to a span.
pub struct TimingContext {
    span: Span,
    start: std::time::Instant,
    _operation: String,
}

impl TimingContext {
    /// Create a new timing context for an operation.
    pub fn new(span: Span, operation: impl Into<String>) -> Self {
        Self {
            span,
            start: std::time::Instant::now(),
            _operation: operation.into(),
        }
    }

    /// Get the elapsed duration.
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// Finish timing and record duration to the span.
    #[allow(clippy::cast_possible_truncation)]
    pub fn finish(self) {
        let dur = self.start.elapsed();
        self.span.record("duration_ms", dur.as_millis() as i64);
    }

    /// Finish with a result, recording success/failure and timing.
    #[allow(clippy::cast_possible_truncation)]
    pub fn finish_with_result<T, E: Display>(self, result: &Result<T, E>) {
        let dur = self.start.elapsed();
        self.span.record("duration_ms", dur.as_millis() as i64);
        self.span.record_result(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instrumented_result_ok() {
        let result: Result<i32, &str> = Ok(42);
        assert!(result.record_to_span().is_ok());
    }

    #[test]
    fn instrumented_result_err() {
        let result: Result<i32, &str> = Err("test error");
        assert!(result.record_to_span().is_err());
    }
}
