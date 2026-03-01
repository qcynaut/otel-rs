//! Modern OpenTelemetry observability helpers for Rust.
//!
//! Provides composable configuration, dynamic layer composition, and
//! async-aware shutdown for traces, logs, and metrics.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use otel_rs::OtelConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut guard = OtelConfig::builder()
//!         .service_name("my-service")
//!         .service_version("1.0.0")
//!         .allow_crate("my_service")
//!         .init()
//!         .await?;
//!
//!     tracing::info!("Hello from my-service!");
//!
//!     // Graceful async shutdown (or let Drop handle it).
//!     guard.shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Error Recording
//!
//! ```rust,ignore
//! use otel_rs::{SpanExt, InstrumentedResult};
//!
//! #[tracing::instrument]
//! async fn work() -> Result<(), MyError> {
//!     // Automatically records errors to the current span:
//!     fallible_call().await.record_to_span()?;
//!
//!     // Or use the macro:
//!     let val = otel_rs::try_record_return!(another_call().await);
//!     Ok(())
//! }
//! ```

pub mod config;
pub(crate) mod env;
pub mod error;
pub mod filter;
#[macro_use]
pub mod macros;
pub mod metrics;
pub mod span;
pub(crate) mod transport;

// ── Re-exports ─────────────────────────────────────────────────────

pub use config::{
    ExporterConfig, ExporterConfigBuilder, LogLevel, MetricsConfig, MetricsConfigBuilder,
    OtelConfig, OtelConfigBuilder, OtlpCredentials, OtlpProtocol, OutputFormat, SamplingStrategy,
    TracingConfig, TracingConfigBuilder,
};
pub use error::{ErrorContext, OtelError, OtelResult};
pub use filter::FilterBuilder;
pub use metrics::Metrics;
use opentelemetry::{KeyValue, trace::TracerProvider};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::{
    Resource,
    logs::SdkLoggerProvider,
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
pub use span::{InstrumentedResult, SpanExt, TimingContext};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{Layer, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::SamplingStrategy as Sampling, filter::build_env_filter};

// ── OtelGuard ──────────────────────────────────────────────────────

/// Guard managing the lifecycle of the observability stack.
///
/// **Keep alive** for the duration of your application. Dropping the
/// guard triggers a synchronous best-effort shutdown. For proper error
/// reporting, call [`shutdown()`](OtelGuard::shutdown) explicitly.
pub struct OtelGuard {
    trace_provider: Option<SdkTracerProvider>,
    log_provider: Option<SdkLoggerProvider>,
    metrics: Option<Metrics>,
    service_name: String,
}

impl OtelGuard {
    /// Get the metrics handle (if metrics are enabled).
    pub const fn metrics(&self) -> Option<&Metrics> {
        self.metrics.as_ref()
    }

    /// Get the service name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Flush all pending telemetry without shutting down.
    pub fn flush(&self) {
        if let Some(ref tp) = self.trace_provider {
            let _ = tp.force_flush();
        }
        if let Some(ref lp) = self.log_provider {
            let _ = lp.force_flush();
        }
    }

    /// Gracefully shut down all telemetry providers.
    ///
    /// This is preferred over relying on [`Drop`] because errors can
    /// be reported. After calling this, the [`Drop`] impl becomes a
    /// no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if any provider fails to shut down.
    pub async fn shutdown(&mut self) -> OtelResult<()> {
        // Take ownership to prevent double-shutdown in Drop.
        if let Some(tp) = self.trace_provider.take() {
            tp.shutdown()
                .map_err(|e| OtelError::init(format!("trace shutdown: {e}")))?;
        }
        if let Some(lp) = self.log_provider.take() {
            lp.shutdown()
                .map_err(|e| OtelError::init(format!("log shutdown: {e}")))?;
        }
        if let Some(m) = self.metrics.take() {
            m.shutdown()?;
        }
        Ok(())
    }
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        // Sync fallback — only runs if async shutdown() was not called.
        if let Some(ref tp) = self.trace_provider {
            let _ = tp.shutdown();
        }
        if let Some(ref lp) = self.log_provider {
            let _ = lp.shutdown();
        }
        if let Some(ref m) = self.metrics {
            let _ = m.shutdown();
        }
    }
}

// ── Builder → init bridge ──────────────────────────────────────────

impl OtelConfigBuilder {
    /// Build configuration and initialize the observability stack.
    ///
    /// # Errors
    ///
    /// Returns an error if exporter building or subscriber
    /// initialization fails.
    pub async fn init(self) -> OtelResult<OtelGuard> {
        init_with_config(self.build()).await
    }
}

// ── Public initialization ──────────────────────────────────────────

/// Initialize the observability stack with a pre-built configuration.
///
/// Prefer [`OtelConfigBuilder::init()`] for a fluent API.
///
/// # Errors
///
/// Returns an error if exporter building or subscriber initialization
/// fails.
pub async fn init_with_config(config: OtelConfig) -> OtelResult<OtelGuard> {
    // ── Build OTel resource ────────────────────────────────────────
    let mut rb = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attribute(KeyValue::new(
            "service.version",
            config.service_version.clone(),
        ))
        .with_attribute(KeyValue::new(
            "deployment.environment.name",
            config.environment.clone(),
        ));

    if let Some(ref ns) = config.service_namespace {
        rb = rb.with_attribute(KeyValue::new("service.namespace", ns.clone()));
    }
    if let Some(ref id) = config.service_instance_id {
        rb = rb.with_attribute(KeyValue::new("service.instance.id", id.clone()));
    }
    for (key, value) in &config.custom_attributes {
        rb = rb.with_attribute(KeyValue::new(key.clone(), value.clone()));
    }

    let resource = rb.build();

    // ── Collect layers dynamically ─────────────────────────────────
    let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> = Vec::new();

    // 1. Global env filter.
    layers.push(Box::new(build_env_filter(&config)));

    // 2. Console output.
    if config.enable_console_output {
        match config.output_format {
            config::OutputFormat::Pretty => {
                layers.push(Box::new(fmt::layer().pretty()));
            }
            config::OutputFormat::Compact => {
                layers.push(Box::new(fmt::layer()));
            }
            config::OutputFormat::Json => {
                layers.push(Box::new(fmt::layer().json()));
            }
        }
    }

    // 3. Trace provider + OpenTelemetry layer.
    let trace_provider = if let Some(ref tc) = config.tracing {
        let exporter = transport::build_span_exporter(&config.exporter)?;

        let sampler = match tc.sampling {
            Sampling::AlwaysOn => Sampler::AlwaysOn,
            Sampling::AlwaysOff => Sampler::AlwaysOff,
            Sampling::TraceIdRatio(r) => Sampler::TraceIdRatioBased(r),
            Sampling::ParentBased => Sampler::ParentBased(Box::new(Sampler::AlwaysOn)),
        };

        let provider = SdkTracerProvider::builder()
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource.clone())
            .with_sampler(sampler)
            .with_batch_exporter(exporter)
            .build();

        let tracer = provider.tracer(config.service_name.clone());
        layers.push(Box::new(OpenTelemetryLayer::new(tracer)));

        Some(provider)
    } else {
        None
    };

    // 4. Log provider + bridge layer.
    let log_provider = if config.logging {
        let exporter = transport::build_log_exporter(&config.exporter)?;

        let provider = SdkLoggerProvider::builder()
            .with_resource(resource.clone())
            .with_batch_exporter(exporter)
            .build();

        layers.push(Box::new(OpenTelemetryTracingBridge::new(&provider)));

        Some(provider)
    } else {
        None
    };

    // 5. Metrics (not a subscriber layer — independent provider).
    let metrics = if config.metrics.is_some() {
        Some(Metrics::new(&config, resource)?)
    } else {
        None
    };

    // ── Initialize the global subscriber ───────────────────────────
    tracing_subscriber::registry()
        .with(layers)
        .try_init()
        .map_err(|_| OtelError::SubscriberAlreadySet)?;

    tracing::info!(
        service.name = %config.service_name,
        service.version = %config.service_version,
        environment = %config.environment,
        "Observability initialized"
    );

    Ok(OtelGuard {
        trace_provider,
        log_provider,
        metrics,
        service_name: config.service_name,
    })
}
