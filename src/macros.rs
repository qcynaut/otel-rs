//! Convenience macros for instrumented operations.

/// Record any error from an expression to the current span.
///
/// Returns the `Result` unchanged — useful for one-liners where you
/// want error tracking without verbose handling.
///
/// # Example
///
/// ```rust,ignore
/// use otel_rs::try_record;
///
/// #[tracing::instrument]
/// async fn my_fn() -> Result<String, MyError> {
///     let value = try_record!(fallible_call().await);
///     Ok(value?)
/// }
/// ```
#[macro_export]
macro_rules! try_record {
    ($expr:expr) => {{
        let result = $expr;
        if let Err(ref e) = result {
            use $crate::span::SpanExt;
            ::tracing::Span::current().record_error(e);
        }
        result
    }};
}

/// Record any error and early-return on `Err` (like `?` with span recording).
///
/// # Example
///
/// ```rust,ignore
/// use otel_rs::try_record_return;
///
/// #[tracing::instrument]
/// async fn my_fn() -> Result<String, MyError> {
///     let value = try_record_return!(fallible_call().await);
///     Ok(format!("Got: {value}"))
/// }
/// ```
#[macro_export]
macro_rules! try_record_return {
    ($expr:expr) => {{
        match $expr {
            Ok(val) => val,
            Err(e) => {
                use $crate::span::SpanExt;
                ::tracing::Span::current().record_error(&e);
                return Err(e.into());
            }
        }
    }};
}
