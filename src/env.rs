//! Standard OTel environment variable parsing.
//!
//! Reads the following variables (all optional, builder values take
//! precedence):
//!
//! | Variable | Maps to |
//! |---|---|
//! | `OTEL_SERVICE_NAME` | service name |
//! | `OTEL_EXPORTER_OTLP_ENDPOINT` | collector endpoint |
//! | `OTEL_EXPORTER_OTLP_PROTOCOL` | `grpc` or `http/protobuf` |
//! | `OTEL_EXPORTER_OTLP_HEADERS` | comma-separated `key=value` pairs |
//! | `OTEL_EXPORTER_OTLP_TIMEOUT` | export timeout in milliseconds |
//! | `OTEL_TRACES_SAMPLER` | sampling strategy name |
//! | `OTEL_TRACES_SAMPLER_ARG` | sampler argument (e.g., ratio) |

use std::{collections::HashMap, time::Duration};

use crate::config::{OtlpProtocol, SamplingStrategy};

/// Parsed OTel environment variables (all optional).
#[derive(Debug, Default)]
pub(crate) struct EnvConfig {
    pub service_name: Option<String>,
    pub endpoint: Option<String>,
    pub protocol: Option<OtlpProtocol>,
    pub headers: Option<HashMap<String, String>>,
    pub timeout: Option<Duration>,
    pub sampler: Option<SamplingStrategy>,
}

/// Read standard OTel environment variables.
pub(crate) fn read_env() -> EnvConfig {
    EnvConfig {
        service_name: std::env::var("OTEL_SERVICE_NAME").ok(),
        endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
        protocol: std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL")
            .ok()
            .and_then(|s| s.parse().ok()),
        headers: parse_headers(),
        timeout: std::env::var("OTEL_EXPORTER_OTLP_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis),
        sampler: parse_sampler(),
    }
}

/// Parse `OTEL_EXPORTER_OTLP_HEADERS` as comma-separated `key=value` pairs.
fn parse_headers() -> Option<HashMap<String, String>> {
    let raw = std::env::var("OTEL_EXPORTER_OTLP_HEADERS").ok()?;
    let mut headers = HashMap::new();
    for pair in raw.split(',') {
        if let Some((k, v)) = pair.split_once('=') {
            headers.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    if headers.is_empty() {
        None
    } else {
        Some(headers)
    }
}

/// Parse `OTEL_TRACES_SAMPLER` and `OTEL_TRACES_SAMPLER_ARG`.
fn parse_sampler() -> Option<SamplingStrategy> {
    let sampler = std::env::var("OTEL_TRACES_SAMPLER").ok()?;
    match sampler.to_lowercase().as_str() {
        "always_on" => Some(SamplingStrategy::AlwaysOn),
        "always_off" => Some(SamplingStrategy::AlwaysOff),
        "traceidratio" => {
            let ratio = std::env::var("OTEL_TRACES_SAMPLER_ARG")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(1.0);
            Some(SamplingStrategy::TraceIdRatio(ratio))
        }
        "parentbased_always_on" | "parentbased" => Some(SamplingStrategy::ParentBased),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_env_without_vars() {
        // In test environment, OTEL_* vars are typically unset.
        let env = read_env();
        // If they happen to be set in CI, just verify the struct is valid.
        let _ = env;
    }

    #[test]
    fn parse_headers_when_unset() {
        // OTEL_EXPORTER_OTLP_HEADERS is not set → None.
        // (This test only works if the var is genuinely unset.)
        if std::env::var("OTEL_EXPORTER_OTLP_HEADERS").is_err() {
            assert!(parse_headers().is_none());
        }
    }
}
