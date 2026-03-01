# AGENTS.md — otel-rs

Knowledge base for AI agents working on this codebase.

## Project Overview

`otel-rs` is a modern OpenTelemetry observability crate for Rust. It provides a single `init()` call that configures traces, logs, and metrics export via OTLP with composable configuration, dynamic subscriber layer composition, and async-aware shutdown.

Dual-licensed under MIT and Apache 2.0.

## Tech Stack

- **Language**: Rust (edition 2024)
- **OpenTelemetry SDK**: `opentelemetry` 0.31, `opentelemetry_sdk` 0.31, `opentelemetry-otlp` 0.31
- **Tracing**: `tracing` 0.1, `tracing-subscriber` 0.3, `tracing-opentelemetry` 0.32
- **gRPC**: `tonic` 0.14 (optional, `grpc` feature)
- **HTTP**: `reqwest` via `opentelemetry-otlp/reqwest-rustls` (optional, `http` feature)
- **Error handling**: `thiserror` 2
- **Base64**: `data-encoding` 2

## Module Map

```
src/
├── lib.rs          # Entry point. OtelGuard, init_with_config(), public re-exports.
│                   # Dynamic layer composition: Vec<Box<dyn Layer<Registry>>>.
│
├── config.rs       # All configuration types and builders.
│                   # Enums: OtlpProtocol, SamplingStrategy, LogLevel, OutputFormat, OtlpCredentials.
│                   # Sub-configs: ExporterConfig, TracingConfig, MetricsConfig (each with builder).
│                   # Main: OtelConfig + OtelConfigBuilder.
│                   # Resolution order: builder values > env vars > hardcoded defaults.
│
├── env.rs          # Internal. Parses standard OTEL_* environment variables.
│                   # Returns EnvConfig struct consumed by OtelConfigBuilder::build().
│                   # Variables: OTEL_SERVICE_NAME, OTEL_EXPORTER_OTLP_ENDPOINT,
│                   # OTEL_EXPORTER_OTLP_PROTOCOL, OTEL_EXPORTER_OTLP_HEADERS,
│                   # OTEL_EXPORTER_OTLP_TIMEOUT, OTEL_TRACES_SAMPLER, OTEL_TRACES_SAMPLER_ARG.
│
├── filter.rs       # Allow-list log filtering. Default: all crates silenced ("off").
│                   # build_env_filter() — used by lib.rs init.
│                   # FilterBuilder — public, for advanced use cases.
│                   # RUST_LOG takes full precedence when set.
│
├── transport.rs    # Internal. Centralized OTLP exporter building.
│                   # build_span_exporter(), build_log_exporter(), build_metric_exporter().
│                   # Handles: credential application (gRPC metadata / HTTP headers),
│                   # TLS config for HTTPS endpoints, feature-gated protocol selection.
│                   # Uses data-encoding for base64 (basic auth).
│
├── metrics.rs      # Thin wrapper: Metrics struct = Arc<SdkMeterProvider> + Meter.
│                   # Convenience: counter(), histogram(), gauge(), up_down_counter().
│                   # shutdown() for graceful provider shutdown.
│
├── error.rs        # OtelError enum (ExporterBuildError, UnknownProtocol, MetricsShutdown,
│                   # Configuration, Initialization, SubscriberAlreadySet).
│                   # OtelResult<T> alias. ErrorContext trait for .context()/.with_context().
│
├── span.rs         # SpanExt trait on tracing::Span — record_error(), record_exception(),
│                   # record_result(), set_ok(), set_error(), set_string_attribute().
│                   # InstrumentedResult trait — record_to_span(), record_to().
│                   # TimingContext — duration tracking with span recording.
│
└── macros.rs       # try_record! — record error to current span, return Result.
                    # try_record_return! — record error and early-return on Err.
```

## Feature Flags

| Feature | Default | What it enables |
|---------|---------|-----------------|
| `tracing` | yes | Span export (`opentelemetry-otlp/trace`) |
| `logs` | yes | Log export (`opentelemetry-otlp/logs`) |
| `metrics` | yes | Metric export (`opentelemetry-otlp/metrics`) |
| `grpc` | yes | gRPC transport via tonic |
| `http` | no | HTTP/protobuf transport via reqwest |

`grpc` and `http` are mutually exclusive at the protocol level but can both be compiled in. The runtime protocol is determined by `ExporterConfig.protocol` (or `OTEL_EXPORTER_OTLP_PROTOCOL`).

## Key Patterns

### Configuration Resolution

```
builder.build() resolves:
  1. Explicit builder values (highest priority)
  2. OTEL_* environment variables (via env::read_env())
  3. Hardcoded defaults (lowest priority)
```

### Layer Composition

`init_with_config()` in `lib.rs` collects layers into a `Vec<Box<dyn Layer<Registry>>>`:
1. EnvFilter (always)
2. Console fmt layer (if enabled — pretty/compact/json)
3. OpenTelemetryLayer (if tracing enabled)
4. OpenTelemetryTracingBridge (if logging enabled)

Applied via `registry().with(layers).try_init()`.

### Shutdown

`OtelGuard` has dual shutdown:
- `shutdown(&mut self) -> OtelResult<()>` — async, takes providers via `.take()`, reports errors.
- `Drop` — sync fallback, only fires if async shutdown wasn't called (providers still `Some`).

### Transport

`transport.rs` has per-exporter functions (not generic). Each function:
1. Matches on protocol (grpc/http)
2. Builds exporter with endpoint + timeout
3. Applies TLS for HTTPS endpoints (gRPC only — tonic `ClientTlsConfig`)
4. Applies credentials (gRPC: tonic metadata, HTTP: request headers)

## Development Commands

```bash
# Check (default features = grpc)
cargo check

# Check with HTTP transport
cargo check --no-default-features --features "tracing,logs,metrics,http"

# Run tests
cargo test

# Clippy
cargo clippy -- -D warnings

# Format check
cargo fmt -- --check
```

## Conventions

- **Error handling**: Use `OtelError` variants, never `unwrap()` in library code. Tests may use `unwrap()`.
- **Builder pattern**: All config structs use owned builders with `#[must_use]` on methods.
- **Visibility**: `pub(crate)` for internal modules (`env`, `transport`). Public API re-exported from `lib.rs`.
- **Feature gating**: Transport code uses `#[cfg(feature = "grpc")]` / `#[cfg(feature = "http")]`. Non-enabled paths return `OtelError::config(...)`.
- **Formatting**: `rustfmt.toml` — 100 char width, 4-space indent, crate-level import granularity.
- **Linting**: `clippy.toml` — cognitive complexity 15, max function lines 80, max args 7.

## Common Tasks

### Adding a new config field
1. Add field to the relevant sub-config struct in `config.rs` (e.g., `TracingConfig`)
2. Add default value in the `Default` impl
3. Add builder method on the corresponding builder struct
4. Wire it up in `lib.rs` `init_with_config()` where the sub-config is consumed

### Adding a new env var
1. Add field to `EnvConfig` in `env.rs`
2. Parse it in `read_env()`
3. Consume it in `OtelConfigBuilder::build()` in `config.rs`

### Adding a new error variant
1. Add variant to `OtelError` in `error.rs`
2. Use it at the call site — `thiserror` handles `Display`/`Error` derives

### Adding a new exporter type
1. Add a `build_*_exporter()` function in `transport.rs` following the existing pattern
2. Wire it into `init_with_config()` in `lib.rs`
