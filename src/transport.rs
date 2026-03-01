//! Centralized OTLP exporter building and credential application.
//!
//! All span, log, and metric exporters are built here with shared
//! credential and TLS handling, eliminating the per-module duplication
//! from the original crate.

use opentelemetry_otlp::WithExportConfig;

use crate::{
    config::{ExporterConfig, OtlpCredentials, OtlpProtocol},
    error::{OtelError, OtelResult},
};

// ── Span exporter ──────────────────────────────────────────────────

/// Build a span exporter from exporter configuration.
pub(crate) fn build_span_exporter(
    config: &ExporterConfig,
) -> OtelResult<opentelemetry_otlp::SpanExporter> {
    use opentelemetry_otlp::SpanExporter;

    match config.protocol {
        OtlpProtocol::Grpc => {
            #[cfg(feature = "grpc")]
            {
                use opentelemetry_otlp::WithTonicConfig;

                let mut b = SpanExporter::builder()
                    .with_tonic()
                    .with_endpoint(&config.endpoint)
                    .with_timeout(config.timeout);

                if config.endpoint.starts_with("https://") {
                    b = b.with_tls_config(
                        tonic::transport::ClientTlsConfig::new().with_webpki_roots(),
                    );
                }

                b = apply_grpc_credentials(b, &config.credentials);
                Ok(b.build()?)
            }
            #[cfg(not(feature = "grpc"))]
            {
                Err(OtelError::config(
                    "gRPC support requires the 'grpc' cargo feature",
                ))
            }
        }
        OtlpProtocol::Http => {
            #[cfg(feature = "http")]
            {
                let endpoint = format!("{}/v1/traces", config.endpoint);
                let mut b = SpanExporter::builder()
                    .with_http()
                    .with_endpoint(endpoint)
                    .with_timeout(config.timeout);

                b = apply_http_credentials(b, &config.credentials);
                Ok(b.build()?)
            }
            #[cfg(not(feature = "http"))]
            {
                Err(OtelError::config(
                    "HTTP support requires the 'http' cargo feature",
                ))
            }
        }
    }
}

// ── Log exporter ───────────────────────────────────────────────────

/// Build a log exporter from exporter configuration.
pub(crate) fn build_log_exporter(
    config: &ExporterConfig,
) -> OtelResult<opentelemetry_otlp::LogExporter> {
    use opentelemetry_otlp::LogExporter;

    match config.protocol {
        OtlpProtocol::Grpc => {
            #[cfg(feature = "grpc")]
            {
                use opentelemetry_otlp::WithTonicConfig;

                let mut b = LogExporter::builder()
                    .with_tonic()
                    .with_endpoint(&config.endpoint)
                    .with_timeout(config.timeout);

                if config.endpoint.starts_with("https://") {
                    b = b.with_tls_config(
                        tonic::transport::ClientTlsConfig::new().with_webpki_roots(),
                    );
                }

                b = apply_grpc_credentials(b, &config.credentials);
                Ok(b.build()?)
            }
            #[cfg(not(feature = "grpc"))]
            {
                Err(OtelError::config(
                    "gRPC support requires the 'grpc' cargo feature",
                ))
            }
        }
        OtlpProtocol::Http => {
            #[cfg(feature = "http")]
            {
                let endpoint = format!("{}/v1/logs", config.endpoint);
                let mut b = LogExporter::builder()
                    .with_http()
                    .with_endpoint(endpoint)
                    .with_timeout(config.timeout);

                b = apply_http_credentials(b, &config.credentials);
                Ok(b.build()?)
            }
            #[cfg(not(feature = "http"))]
            {
                Err(OtelError::config(
                    "HTTP support requires the 'http' cargo feature",
                ))
            }
        }
    }
}

// ── Metric exporter ────────────────────────────────────────────────

/// Build a metric exporter from exporter configuration.
pub(crate) fn build_metric_exporter(
    config: &ExporterConfig,
) -> OtelResult<opentelemetry_otlp::MetricExporter> {
    use opentelemetry_otlp::MetricExporter;

    match config.protocol {
        OtlpProtocol::Grpc => {
            #[cfg(feature = "grpc")]
            {
                use opentelemetry_otlp::WithTonicConfig;

                let mut b = MetricExporter::builder()
                    .with_tonic()
                    .with_endpoint(&config.endpoint)
                    .with_timeout(config.timeout);

                if config.endpoint.starts_with("https://") {
                    b = b.with_tls_config(
                        tonic::transport::ClientTlsConfig::new().with_webpki_roots(),
                    );
                }

                b = apply_grpc_credentials(b, &config.credentials);
                Ok(b.build()?)
            }
            #[cfg(not(feature = "grpc"))]
            {
                Err(OtelError::config(
                    "gRPC support requires the 'grpc' cargo feature",
                ))
            }
        }
        OtlpProtocol::Http => {
            #[cfg(feature = "http")]
            {
                let endpoint = format!("{}/v1/metrics", config.endpoint);
                let mut b = MetricExporter::builder()
                    .with_http()
                    .with_endpoint(endpoint)
                    .with_timeout(config.timeout);

                b = apply_http_credentials(b, &config.credentials);
                Ok(b.build()?)
            }
            #[cfg(not(feature = "http"))]
            {
                Err(OtelError::config(
                    "HTTP support requires the 'http' cargo feature",
                ))
            }
        }
    }
}

// ── Credential application (gRPC) ─────────────────────────────────

#[cfg(feature = "grpc")]
fn apply_grpc_credentials<T: opentelemetry_otlp::WithTonicConfig>(
    builder: T,
    credentials: &OtlpCredentials,
) -> T {
    use tonic::metadata::{Ascii, MetadataKey, MetadataMap, MetadataValue};

    match credentials {
        OtlpCredentials::None => builder,
        OtlpCredentials::Bearer(token) => {
            let mut metadata = MetadataMap::new();
            if let Ok(value) = format!("Bearer {token}").parse::<MetadataValue<Ascii>>() {
                metadata.insert("authorization", value);
            }
            builder.with_metadata(metadata)
        }
        OtlpCredentials::Basic { username, password } => {
            let mut metadata = MetadataMap::new();
            let encoded = data_encoding::BASE64.encode(format!("{username}:{password}").as_bytes());
            if let Ok(value) = format!("Basic {encoded}").parse::<MetadataValue<Ascii>>() {
                metadata.insert("authorization", value);
            }
            builder.with_metadata(metadata)
        }
        OtlpCredentials::Headers(headers) => {
            let mut metadata = MetadataMap::new();
            for (key, value) in headers {
                if let (Ok(k), Ok(v)) = (
                    key.parse::<MetadataKey<Ascii>>(),
                    value.parse::<MetadataValue<Ascii>>(),
                ) {
                    metadata.insert(k, v);
                }
            }
            builder.with_metadata(metadata)
        }
    }
}

// ── Credential application (HTTP) ──────────────────────────────────

#[cfg(feature = "http")]
fn apply_http_credentials<T: opentelemetry_otlp::WithHttpConfig>(
    builder: T,
    credentials: &OtlpCredentials,
) -> T {
    use std::collections::HashMap;

    match credentials {
        OtlpCredentials::None => builder,
        OtlpCredentials::Bearer(token) => {
            let mut headers = HashMap::new();
            headers.insert("authorization".to_string(), format!("Bearer {token}"));
            builder.with_headers(headers)
        }
        OtlpCredentials::Basic { username, password } => {
            let mut headers = HashMap::new();
            let encoded = data_encoding::BASE64.encode(format!("{username}:{password}").as_bytes());
            headers.insert("authorization".to_string(), format!("Basic {encoded}"));
            builder.with_headers(headers)
        }
        OtlpCredentials::Headers(headers) => builder.with_headers(headers.clone()),
    }
}
