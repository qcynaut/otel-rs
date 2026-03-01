//! Configuration for the observability stack.
//!
//! Provides composable sub-configurations with builder patterns for
//! fine-grained control over exporter, tracing, and metrics settings.
//!
//! # Example
//!
//! ```rust,ignore
//! use otel_rs::config::*;
//!
//! let _guard = OtelConfig::builder()
//!     .service_name("my-service")
//!     .service_version("1.0.0")
//!     .exporter(ExporterConfig::builder()
//!         .endpoint("https://otel.example.com:4317")
//!         .bearer_token("your-api-key")
//!         .build())
//!     .tracing(TracingConfig::builder()
//!         .sampling(SamplingStrategy::TraceIdRatio(0.1))
//!         .build())
//!     .allow_crate("my_service")
//!     .log_level(LogLevel::Info)
//!     .output_format(OutputFormat::Json)
//!     .init()
//!     .await?;
//! ```

use std::time::Duration;

// ── Enums ──────────────────────────────────────────────────────────

/// OTLP protocol for exporting telemetry data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OtlpProtocol {
    /// gRPC protocol (default, more efficient for high-volume telemetry).
    #[default]
    Grpc,
    /// HTTP/protobuf protocol (better firewall compatibility).
    Http,
}

impl OtlpProtocol {
    /// Returns the string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Grpc => "grpc",
            Self::Http => "http",
        }
    }
}

impl std::str::FromStr for OtlpProtocol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "grpc" => Ok(Self::Grpc),
            "http" | "http/protobuf" => Ok(Self::Http),
            other => Err(format!("unknown OTLP protocol: {other}")),
        }
    }
}

impl std::fmt::Display for OtlpProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Sampling strategy for traces.
#[derive(Debug, Clone, Copy, Default)]
pub enum SamplingStrategy {
    /// Always sample all traces (default).
    #[default]
    AlwaysOn,
    /// Never sample.
    AlwaysOff,
    /// Sample based on trace ID ratio (0.0 to 1.0).
    TraceIdRatio(f64),
    /// Sample based on parent span decision.
    ParentBased,
}

/// Log level for filtering output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Returns the string representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            other => Err(format!("unknown log level: {other}")),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Output format for console/stdout logs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable pretty format (default).
    #[default]
    Pretty,
    /// Compact single-line format.
    Compact,
    /// JSON format for structured logging.
    Json,
}

/// OTLP authentication credentials.
#[derive(Debug, Clone, Default)]
pub enum OtlpCredentials {
    /// No authentication (default).
    #[default]
    None,
    /// Bearer token (`Authorization: Bearer <token>`).
    Bearer(String),
    /// Basic auth (`Authorization: Basic base64(user:pass)`).
    Basic {
        /// Username.
        username: String,
        /// Password.
        password: String,
    },
    /// Custom headers.
    Headers(std::collections::HashMap<String, String>),
}

// ── Sub-configs ────────────────────────────────────────────────────

/// OTLP exporter connection configuration.
#[derive(Debug, Clone)]
pub struct ExporterConfig {
    pub(crate) endpoint: String,
    pub(crate) protocol: OtlpProtocol,
    pub(crate) timeout: Duration,
    pub(crate) credentials: OtlpCredentials,
}

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4317".to_string(),
            protocol: OtlpProtocol::default(),
            timeout: Duration::from_secs(10),
            credentials: OtlpCredentials::None,
        }
    }
}

impl ExporterConfig {
    /// Create a builder.
    #[must_use]
    pub fn builder() -> ExporterConfigBuilder {
        ExporterConfigBuilder::new()
    }
}

/// Builder for [`ExporterConfig`].
#[derive(Debug, Clone)]
pub struct ExporterConfigBuilder {
    config: ExporterConfig,
}

impl Default for ExporterConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExporterConfigBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ExporterConfig::default(),
        }
    }

    /// Set the OTLP collector endpoint.
    #[must_use]
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.config.endpoint = endpoint.into();
        self
    }

    /// Set the OTLP protocol.
    #[must_use]
    pub const fn protocol(mut self, protocol: OtlpProtocol) -> Self {
        self.config.protocol = protocol;
        self
    }

    /// Set the export timeout.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set bearer token authentication.
    #[must_use]
    pub fn bearer_token(mut self, token: impl Into<String>) -> Self {
        self.config.credentials = OtlpCredentials::Bearer(token.into());
        self
    }

    /// Set basic authentication.
    #[must_use]
    pub fn basic_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.config.credentials = OtlpCredentials::Basic {
            username: username.into(),
            password: password.into(),
        };
        self
    }

    /// Add a custom header. Can be called multiple times.
    #[must_use]
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        match &mut self.config.credentials {
            OtlpCredentials::Headers(h) => {
                h.insert(key.into(), value.into());
            }
            _ => {
                let mut h = std::collections::HashMap::new();
                h.insert(key.into(), value.into());
                self.config.credentials = OtlpCredentials::Headers(h);
            }
        }
        self
    }

    /// Set custom headers, replacing any existing credentials.
    #[must_use]
    pub fn headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.config.credentials = OtlpCredentials::Headers(headers);
        self
    }

    /// Build the exporter configuration.
    #[must_use]
    pub fn build(self) -> ExporterConfig {
        self.config
    }
}

/// Tracing-specific configuration.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub(crate) sampling: SamplingStrategy,
    pub(crate) record_exceptions: bool,
    pub(crate) exception_field_limit: usize,
    pub(crate) batch_schedule_delay: Duration,
    pub(crate) max_export_batch_size: usize,
    pub(crate) max_queue_size: usize,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            sampling: SamplingStrategy::default(),
            record_exceptions: true,
            exception_field_limit: 1024,
            batch_schedule_delay: Duration::from_secs(5),
            max_export_batch_size: 512,
            max_queue_size: 2048,
        }
    }
}

impl TracingConfig {
    /// Create a builder.
    #[must_use]
    pub fn builder() -> TracingConfigBuilder {
        TracingConfigBuilder::new()
    }
}

/// Builder for [`TracingConfig`].
#[derive(Debug, Clone)]
pub struct TracingConfigBuilder {
    config: TracingConfig,
}

impl Default for TracingConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TracingConfigBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TracingConfig::default(),
        }
    }

    /// Set the sampling strategy.
    #[must_use]
    pub const fn sampling(mut self, strategy: SamplingStrategy) -> Self {
        self.config.sampling = strategy;
        self
    }

    /// Enable or disable automatic exception recording.
    #[must_use]
    pub const fn record_exceptions(mut self, enabled: bool) -> Self {
        self.config.record_exceptions = enabled;
        self
    }

    /// Set the maximum length for exception fields.
    #[must_use]
    pub const fn exception_field_limit(mut self, limit: usize) -> Self {
        self.config.exception_field_limit = limit;
        self
    }

    /// Set the batch schedule delay for trace export.
    #[must_use]
    pub const fn batch_schedule_delay(mut self, delay: Duration) -> Self {
        self.config.batch_schedule_delay = delay;
        self
    }

    /// Set the maximum export batch size.
    #[must_use]
    pub const fn max_export_batch_size(mut self, size: usize) -> Self {
        self.config.max_export_batch_size = size;
        self
    }

    /// Set the maximum queue size.
    #[must_use]
    pub const fn max_queue_size(mut self, size: usize) -> Self {
        self.config.max_queue_size = size;
        self
    }

    /// Build the tracing configuration.
    #[must_use]
    pub fn build(self) -> TracingConfig {
        self.config
    }
}

/// Metrics-specific configuration.
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub(crate) export_interval: Duration,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            export_interval: Duration::from_secs(60),
        }
    }
}

impl MetricsConfig {
    /// Create a builder.
    #[must_use]
    pub fn builder() -> MetricsConfigBuilder {
        MetricsConfigBuilder::new()
    }
}

/// Builder for [`MetricsConfig`].
#[derive(Debug, Clone)]
pub struct MetricsConfigBuilder {
    config: MetricsConfig,
}

impl Default for MetricsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsConfigBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: MetricsConfig::default(),
        }
    }

    /// Set the metrics export interval.
    #[must_use]
    pub const fn export_interval(mut self, interval: Duration) -> Self {
        self.config.export_interval = interval;
        self
    }

    /// Build the metrics configuration.
    #[must_use]
    pub fn build(self) -> MetricsConfig {
        self.config
    }
}

// ── Main config ────────────────────────────────────────────────────

/// Resolved configuration for the observability stack.
///
/// Construct via [`OtelConfigBuilder`].
#[derive(Debug, Clone)]
pub struct OtelConfig {
    // Service identity
    pub(crate) service_name: String,
    pub(crate) service_version: String,
    pub(crate) environment: String,
    pub(crate) service_namespace: Option<String>,
    pub(crate) service_instance_id: Option<String>,

    // Exporter
    pub(crate) exporter: ExporterConfig,

    // Feature toggles & sub-configs
    /// `Some(config)` = tracing enabled, `None` = disabled.
    pub(crate) tracing: Option<TracingConfig>,
    /// Whether OTLP log export is enabled.
    pub(crate) logging: bool,
    /// `Some(config)` = metrics enabled, `None` = disabled.
    pub(crate) metrics: Option<MetricsConfig>,

    // Console
    pub(crate) enable_console_output: bool,
    pub(crate) log_level: LogLevel,
    pub(crate) output_format: OutputFormat,

    // Filtering
    pub(crate) allowed_crates: Vec<String>,
    pub(crate) custom_filters: Vec<String>,

    // Custom resource attributes
    pub(crate) custom_attributes: Vec<(String, String)>,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            service_name: "unknown-service".to_string(),
            service_version: "0.0.0".to_string(),
            environment: "development".to_string(),
            service_namespace: None,
            service_instance_id: None,
            exporter: ExporterConfig::default(),
            tracing: Some(TracingConfig::default()),
            logging: true,
            metrics: Some(MetricsConfig::default()),
            enable_console_output: true,
            log_level: LogLevel::Info,
            output_format: OutputFormat::Pretty,
            allowed_crates: Vec::new(),
            custom_filters: Vec::new(),
            custom_attributes: Vec::new(),
        }
    }
}

impl OtelConfig {
    /// Create a new builder.
    #[must_use]
    pub fn builder() -> OtelConfigBuilder {
        OtelConfigBuilder::new()
    }
}

/// Builder for [`OtelConfig`].
///
/// Resolution order: **builder values > env vars > hardcoded defaults**.
///
/// Standard OTel environment variables (`OTEL_SERVICE_NAME`,
/// `OTEL_EXPORTER_OTLP_ENDPOINT`, etc.) are read automatically
/// and applied as defaults that builder methods can override.
///
/// # Example
///
/// ```rust,ignore
/// let _guard = OtelConfig::builder()
///     .service_name("my-service")
///     .service_version("1.0.0")
///     .allow_crate("my_service")
///     .init()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct OtelConfigBuilder {
    service_name: Option<String>,
    service_version: Option<String>,
    environment: Option<String>,
    service_namespace: Option<String>,
    service_instance_id: Option<String>,
    exporter: Option<ExporterConfig>,
    /// `None` = not set (use defaults), `Some(None)` = disabled,
    /// `Some(Some(c))` = custom config.
    tracing: Option<Option<TracingConfig>>,
    logging: Option<bool>,
    metrics: Option<Option<MetricsConfig>>,
    enable_console_output: Option<bool>,
    log_level: Option<LogLevel>,
    output_format: Option<OutputFormat>,
    allowed_crates: Vec<String>,
    custom_filters: Vec<String>,
    custom_attributes: Vec<(String, String)>,
}

impl Default for OtelConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OtelConfigBuilder {
    /// Create a new builder. All fields start unset and will fall back
    /// to environment variables, then hardcoded defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            service_name: None,
            service_version: None,
            environment: None,
            service_namespace: None,
            service_instance_id: None,
            exporter: None,
            tracing: None,
            logging: None,
            metrics: None,
            enable_console_output: None,
            log_level: None,
            output_format: None,
            allowed_crates: Vec::new(),
            custom_filters: Vec::new(),
            custom_attributes: Vec::new(),
        }
    }

    // ── Service identity ───────────────────────────────────────────

    /// Set the service name.
    #[must_use]
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    /// Set the service version.
    #[must_use]
    pub fn service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = Some(version.into());
        self
    }

    /// Set the deployment environment (e.g., `"production"`, `"staging"`).
    #[must_use]
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    /// Set the service namespace for grouping related services.
    #[must_use]
    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.service_namespace = Some(ns.into());
        self
    }

    /// Set a unique instance identifier for this service instance.
    #[must_use]
    pub fn instance_id(mut self, id: impl Into<String>) -> Self {
        self.service_instance_id = Some(id.into());
        self
    }

    // ── Sub-configs ────────────────────────────────────────────────

    /// Set the exporter configuration. Overrides any env var defaults
    /// for endpoint, protocol, timeout, and credentials.
    #[must_use]
    pub fn exporter(mut self, config: ExporterConfig) -> Self {
        self.exporter = Some(config);
        self
    }

    /// Set the tracing configuration. Implicitly enables tracing.
    #[must_use]
    pub fn tracing(mut self, config: TracingConfig) -> Self {
        self.tracing = Some(Some(config));
        self
    }

    /// Disable distributed tracing.
    #[must_use]
    pub fn disable_tracing(mut self) -> Self {
        self.tracing = Some(None);
        self
    }

    /// Enable or disable OTLP log export.
    #[must_use]
    pub fn logging(mut self, enabled: bool) -> Self {
        self.logging = Some(enabled);
        self
    }

    /// Set the metrics configuration. Implicitly enables metrics.
    #[must_use]
    pub fn metrics(mut self, config: MetricsConfig) -> Self {
        self.metrics = Some(Some(config));
        self
    }

    /// Disable metrics.
    #[must_use]
    pub fn disable_metrics(mut self) -> Self {
        self.metrics = Some(None);
        self
    }

    // ── Console & filtering ────────────────────────────────────────

    /// Enable or disable console/stdout output.
    #[must_use]
    pub fn console_output(mut self, enabled: bool) -> Self {
        self.enable_console_output = Some(enabled);
        self
    }

    /// Set the minimum log level.
    #[must_use]
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.log_level = Some(level);
        self
    }

    /// Set the console output format.
    #[must_use]
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = Some(format);
        self
    }

    /// Allow a crate to emit logs at the configured level.
    ///
    /// By default all crates are silenced. Call this for each crate
    /// whose logs you want to see.
    #[must_use]
    pub fn allow_crate(mut self, name: impl Into<String>) -> Self {
        self.allowed_crates.push(name.into());
        self
    }

    /// Allow multiple crates to emit logs.
    #[must_use]
    pub fn allow_crates(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for name in names {
            self.allowed_crates.push(name.into());
        }
        self
    }

    /// Add a custom filter directive (tracing-subscriber `EnvFilter` syntax).
    #[must_use]
    pub fn custom_filter(mut self, directive: impl Into<String>) -> Self {
        self.custom_filters.push(directive.into());
        self
    }

    /// Add a custom resource attribute.
    #[must_use]
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_attributes.push((key.into(), value.into()));
        self
    }

    // ── Build ──────────────────────────────────────────────────────

    /// Build the resolved configuration.
    ///
    /// Resolution: builder values → env vars → hardcoded defaults.
    #[must_use]
    pub fn build(self) -> OtelConfig {
        let env = crate::env::read_env();

        // Exporter: if explicitly set, use as-is; otherwise merge env.
        let exporter = self.exporter.unwrap_or_else(|| {
            let mut exp = ExporterConfig::default();
            if let Some(endpoint) = env.endpoint {
                exp.endpoint = endpoint;
            }
            if let Some(protocol) = env.protocol {
                exp.protocol = protocol;
            }
            if let Some(timeout) = env.timeout {
                exp.timeout = timeout;
            }
            if let Some(headers) = env.headers {
                exp.credentials = OtlpCredentials::Headers(headers);
            }
            exp
        });

        // Tracing: not-set → defaults + env sampler, disabled → None.
        let tracing = match self.tracing {
            Some(t) => t,
            None => {
                let mut tc = TracingConfig::default();
                if let Some(sampler) = env.sampler {
                    tc.sampling = sampler;
                }
                Some(tc)
            }
        };

        // Metrics.
        let metrics = match self.metrics {
            Some(m) => m,
            None => Some(MetricsConfig::default()),
        };

        OtelConfig {
            service_name: self
                .service_name
                .or(env.service_name)
                .unwrap_or_else(|| "unknown-service".to_string()),
            service_version: self.service_version.unwrap_or_else(|| "0.0.0".to_string()),
            environment: self
                .environment
                .unwrap_or_else(|| "development".to_string()),
            service_namespace: self.service_namespace,
            service_instance_id: self.service_instance_id,
            exporter,
            tracing,
            logging: self.logging.unwrap_or(true),
            metrics,
            enable_console_output: self.enable_console_output.unwrap_or(true),
            log_level: self.log_level.unwrap_or_default(),
            output_format: self.output_format.unwrap_or_default(),
            allowed_crates: self.allowed_crates,
            custom_filters: self.custom_filters,
            custom_attributes: self.custom_attributes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let config = OtelConfig::builder().build();
        assert_eq!(config.service_name, "unknown-service");
        assert!(config.tracing.is_some());
        assert!(config.logging);
        assert!(config.metrics.is_some());
        assert!(config.enable_console_output);
    }

    #[test]
    fn builder_with_service_info() {
        let config = OtelConfig::builder()
            .service_name("test-svc")
            .service_version("1.0.0")
            .environment("test")
            .build();

        assert_eq!(config.service_name, "test-svc");
        assert_eq!(config.service_version, "1.0.0");
        assert_eq!(config.environment, "test");
    }

    #[test]
    fn builder_with_exporter() {
        let config = OtelConfig::builder()
            .exporter(
                ExporterConfig::builder()
                    .endpoint("https://otel.example.com:4317")
                    .bearer_token("my-token")
                    .build(),
            )
            .build();

        assert_eq!(config.exporter.endpoint, "https://otel.example.com:4317");
        assert!(matches!(
            config.exporter.credentials,
            OtlpCredentials::Bearer(_)
        ));
    }

    #[test]
    fn builder_disable_tracing() {
        let config = OtelConfig::builder().disable_tracing().build();
        assert!(config.tracing.is_none());
    }

    #[test]
    fn builder_disable_metrics() {
        let config = OtelConfig::builder().disable_metrics().build();
        assert!(config.metrics.is_none());
    }

    #[test]
    fn exporter_header_accumulation() {
        let exp = ExporterConfig::builder()
            .header("x-api-key", "abc")
            .header("x-team", "eng")
            .build();

        match exp.credentials {
            OtlpCredentials::Headers(h) => {
                assert_eq!(h.get("x-api-key").unwrap(), "abc");
                assert_eq!(h.get("x-team").unwrap(), "eng");
            }
            _ => panic!("expected Headers"),
        }
    }

    #[test]
    fn protocol_parsing() {
        assert_eq!("grpc".parse::<OtlpProtocol>().unwrap(), OtlpProtocol::Grpc);
        assert_eq!("http".parse::<OtlpProtocol>().unwrap(), OtlpProtocol::Http);
        assert_eq!(
            "http/protobuf".parse::<OtlpProtocol>().unwrap(),
            OtlpProtocol::Http
        );
        assert!("invalid".parse::<OtlpProtocol>().is_err());
    }

    #[test]
    fn log_level_parsing() {
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("WARNING".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }
}
