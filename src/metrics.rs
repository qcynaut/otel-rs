//! Thin metrics wrapper around the OpenTelemetry SDK.
//!
//! Provides a [`Metrics`] handle for creating instruments and
//! managing the meter provider lifecycle.

use std::sync::Arc;

use opentelemetry::metrics::{Counter, Gauge, Histogram, Meter, MeterProvider, UpDownCounter};
use opentelemetry_sdk::{
    Resource,
    metrics::{PeriodicReader, SdkMeterProvider},
};

use crate::{
    config::OtelConfig,
    error::{OtelError, OtelResult},
};

/// Metrics provider handle.
///
/// Holds the underlying [`SdkMeterProvider`] and a default [`Meter`].
/// Clone is cheap (provider is `Arc`-wrapped).
#[derive(Clone)]
pub struct Metrics {
    provider: Arc<SdkMeterProvider>,
    meter: Meter,
}

impl Metrics {
    /// Create a new metrics instance from configuration and resource.
    pub(crate) fn new(config: &OtelConfig, resource: Resource) -> OtelResult<Self> {
        let metrics_config = config
            .metrics
            .as_ref()
            .ok_or_else(|| OtelError::config("metrics not configured"))?;

        let exporter = crate::transport::build_metric_exporter(&config.exporter)?;

        let reader = PeriodicReader::builder(exporter)
            .with_interval(metrics_config.export_interval)
            .build();

        let provider = SdkMeterProvider::builder()
            .with_resource(resource)
            .with_reader(reader)
            .build();

        let meter = provider.meter("otel-rs");

        Ok(Self {
            provider: Arc::new(provider),
            meter,
        })
    }

    /// Get the underlying [`Meter`] for creating custom instruments.
    pub const fn meter(&self) -> &Meter {
        &self.meter
    }

    // ── Convenience constructors ───────────────────────────────────

    /// Create a `u64` counter.
    pub fn counter(&self, name: &'static str) -> Counter<u64> {
        self.meter.u64_counter(name).build()
    }

    /// Create an `f64` counter.
    pub fn f64_counter(&self, name: &'static str) -> Counter<f64> {
        self.meter.f64_counter(name).build()
    }

    /// Create an `f64` histogram.
    pub fn histogram(&self, name: &'static str) -> Histogram<f64> {
        self.meter.f64_histogram(name).build()
    }

    /// Create a `u64` gauge.
    pub fn gauge(&self, name: &'static str) -> Gauge<u64> {
        self.meter.u64_gauge(name).build()
    }

    /// Create an `f64` gauge.
    pub fn f64_gauge(&self, name: &'static str) -> Gauge<f64> {
        self.meter.f64_gauge(name).build()
    }

    /// Create an `i64` up-down counter.
    pub fn up_down_counter(&self, name: &'static str) -> UpDownCounter<i64> {
        self.meter.i64_up_down_counter(name).build()
    }

    // ── Lifecycle ──────────────────────────────────────────────────

    /// Shutdown the metrics provider gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider fails to shut down.
    pub fn shutdown(&self) -> OtelResult<()> {
        self.provider
            .shutdown()
            .map_err(|e| OtelError::MetricsShutdown(e.to_string()))
    }
}
