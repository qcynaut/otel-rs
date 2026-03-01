# otel-rs

Modern, composable OpenTelemetry observability helpers for Rust.

Provides a single `init()` call that sets up **traces**, **logs**, and **metrics** export via OTLP, with dynamic subscriber layer composition, feature-gated transport selection, standard OTel env var support, and async-aware shutdown.

## Quick Start

```rust
use otel_rs::OtelConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut guard = OtelConfig::builder()
        .service_name("my-service")
        .service_version("1.0.0")
        .allow_crate("my_service")
        .init()
        .await?;

    tracing::info!("Hello from my-service!");

    guard.shutdown().await?;
    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
otel-rs = "0.1"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `tracing` | ✅ | Distributed trace export |
| `logs` | ✅ | Log export via OTLP |
| `metrics` | ✅ | Metrics export via OTLP |
| `grpc` | ✅ | gRPC transport (tonic) |
| `http` | ❌ | HTTP/protobuf transport (reqwest) |

To use HTTP instead of gRPC:

```toml
[dependencies]
otel-rs = { version = "0.1", default-features = false, features = ["tracing", "logs", "metrics", "http"] }
```

## Configuration

### Composable Builder API

Configuration is split into composable sub-configs, each with its own builder:

```rust
use otel_rs::{
    OtelConfig, ExporterConfig, TracingConfig, MetricsConfig,
    OtlpProtocol, SamplingStrategy, LogLevel, OutputFormat,
};
use std::time::Duration;

let mut guard = OtelConfig::builder()
    // Service identity
    .service_name("my-service")
    .service_version("1.0.0")
    .environment("production")
    .namespace("backend")

    // Exporter (endpoint, auth, protocol, timeout)
    .exporter(ExporterConfig::builder()
        .endpoint("https://otel.example.com:4317")
        .bearer_token("your-api-key")
        .timeout(Duration::from_secs(30))
        .build())

    // Tracing (sampling, batching)
    .tracing(TracingConfig::builder()
        .sampling(SamplingStrategy::TraceIdRatio(0.1))
        .batch_schedule_delay(Duration::from_secs(5))
        .build())

    // Metrics (export interval)
    .metrics(MetricsConfig::builder()
        .export_interval(Duration::from_secs(30))
        .build())

    // Console & filtering
    .log_level(LogLevel::Info)
    .output_format(OutputFormat::Json)
    .allow_crate("my_service")
    .allow_crate("my_lib")

    // Custom resource attributes
    .attribute("deployment.region", "us-east-1")

    .init()
    .await?;
```

### Environment Variables

Standard OTel environment variables are respected as defaults. Builder values always take precedence.

| Variable | Effect |
|----------|--------|
| `OTEL_SERVICE_NAME` | Service name |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Collector endpoint |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `grpc` or `http/protobuf` |
| `OTEL_EXPORTER_OTLP_HEADERS` | Comma-separated `key=value` pairs |
| `OTEL_EXPORTER_OTLP_TIMEOUT` | Timeout in milliseconds |
| `OTEL_TRACES_SAMPLER` | `always_on`, `always_off`, `traceidratio`, `parentbased_always_on` |
| `OTEL_TRACES_SAMPLER_ARG` | Sampler argument (e.g., ratio `0.1`) |
| `RUST_LOG` | Full override of log filtering |

### Authentication

```rust
// Bearer token (Honeycomb, Grafana Cloud, etc.)
ExporterConfig::builder()
    .bearer_token("your-api-key")
    .build()

// Basic auth
ExporterConfig::builder()
    .basic_auth("user", "password")
    .build()

// Custom headers
ExporterConfig::builder()
    .header("x-api-key", "abc")
    .header("x-team-id", "eng")
    .build()
```

### Disabling Features

```rust
OtelConfig::builder()
    .service_name("lightweight")
    .disable_tracing()       // No span export
    .disable_metrics()       // No metrics export
    .logging(false)          // No log export
    .console_output(false)   // No stdout
    .init()
    .await?;
```

## Error Recording

Errors are automatically recorded to spans using extension traits and macros:

```rust
use otel_rs::{SpanExt, InstrumentedResult};
use tracing::Span;

#[tracing::instrument]
async fn work() -> Result<String, MyError> {
    // Option 1: Extension trait — records error and passes through
    let data = fetch_data().await.record_to_span()?;

    // Option 2: Macro — records error, early-returns on Err
    let parsed = otel_rs::try_record_return!(parse(data));

    // Option 3: Explicit recording
    let result = validate(&parsed);
    Span::current().record_result(&result);

    Ok(parsed)
}
```

## Metrics

When metrics are enabled, the guard provides a `Metrics` handle:

```rust
use opentelemetry::KeyValue;

let guard = OtelConfig::builder()
    .service_name("my-service")
    .init()
    .await?;

if let Some(metrics) = guard.metrics() {
    let counter = metrics.counter("requests_total");
    counter.add(1, &[KeyValue::new("method", "GET")]);

    let histogram = metrics.histogram("request_duration_ms");
    histogram.record(42.5, &[]);

    let gauge = metrics.gauge("active_connections");
    gauge.record(10, &[]);
}
```

## Shutdown

`OtelGuard` supports both async and sync shutdown:

```rust
// Preferred: async shutdown with error reporting
guard.shutdown().await?;

// Fallback: Drop triggers sync best-effort shutdown
drop(guard);
```

Always prefer the async path — it flushes pending telemetry and reports errors. The `Drop` impl is a safety net for cases where async shutdown isn't called.

## Log Filtering

Uses an **allow-list** approach (default: all crates silenced):

```rust
OtelConfig::builder()
    .log_level(LogLevel::Debug)
    .allow_crate("my_app")          // my_app=debug
    .allow_crate("my_lib")          // my_lib=debug
    .custom_filter("hyper=warn")    // Override for specific crate
    .init()
    .await?;
```

For advanced filtering, use `FilterBuilder` directly:

```rust
use otel_rs::FilterBuilder;

let filter = FilterBuilder::new()
    .default_level("info")
    .allow("my_app")
    .allow_at("my_lib", "trace")
    .directive("tokio=warn")
    .build();
```

`RUST_LOG` takes full precedence when set.

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
