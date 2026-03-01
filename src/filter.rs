//! Allow-list based log filtering.
//!
//! By default all crates are silenced. Only explicitly allowed crates
//! will have their logs emitted at the configured level.

use tracing_subscriber::filter::EnvFilter;

use crate::config::{LogLevel, OtelConfig};

/// Build an [`EnvFilter`] from the resolved configuration.
///
/// Creates an allow-list filter:
/// 1. Defaults to `off` — all crates are silenced
/// 2. Only crates in `allowed_crates` emit logs at the configured level
/// 3. Custom filter directives are applied last
/// 4. `RUST_LOG` env var takes full precedence if set
pub fn build_env_filter(config: &OtelConfig) -> EnvFilter {
    let mut directives = vec!["off".to_string()];

    let level = config.log_level.as_str();
    for crate_name in &config.allowed_crates {
        directives.push(format!("{crate_name}={level}"));
    }

    for custom in &config.custom_filters {
        directives.push(custom.clone());
    }

    let filter_string = directives.join(",");

    // RUST_LOG takes full precedence when set.
    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::try_new(&filter_string).unwrap_or_else(|_| EnvFilter::new("off"))
    })
}

/// Fine-grained filter builder for advanced use cases.
///
/// # Example
///
/// ```rust,ignore
/// use otel_rs::filter::FilterBuilder;
///
/// let filter = FilterBuilder::new()
///     .default_level("info")
///     .allow("my_app")
///     .allow_at("my_lib", "trace")
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct FilterBuilder {
    default_level: String,
    allowed: Vec<(String, String)>,
    directives: Vec<String>,
}

impl FilterBuilder {
    /// Create a new filter builder (`info` default level).
    #[must_use]
    pub fn new() -> Self {
        Self {
            default_level: "info".to_string(),
            allowed: Vec::new(),
            directives: Vec::new(),
        }
    }

    /// Set the default level for allowed crates.
    #[must_use]
    pub fn default_level(mut self, level: impl Into<String>) -> Self {
        self.default_level = level.into();
        self
    }

    /// Allow a crate at the default level.
    #[must_use]
    pub fn allow(mut self, crate_name: impl Into<String>) -> Self {
        let name = crate_name.into();
        let level = self.default_level.clone();
        self.allowed.push((name, level));
        self
    }

    /// Allow a crate at a specific level.
    #[must_use]
    pub fn allow_at(mut self, crate_name: impl Into<String>, level: impl Into<String>) -> Self {
        self.allowed.push((crate_name.into(), level.into()));
        self
    }

    /// Add a raw filter directive.
    #[must_use]
    pub fn directive(mut self, directive: impl Into<String>) -> Self {
        self.directives.push(directive.into());
        self
    }

    /// Build the [`EnvFilter`].
    #[must_use]
    pub fn build(self) -> EnvFilter {
        let mut all = vec!["off".to_string()];

        for (name, level) in self.allowed {
            all.push(format!("{name}={level}"));
        }

        all.extend(self.directives);

        let filter_string = all.join(",");
        EnvFilter::try_new(&filter_string).unwrap_or_else(|_| EnvFilter::new("off"))
    }
}

/// Convenience: convert a [`LogLevel`] to its filter directive string.
pub const fn level_to_directive(level: LogLevel) -> &'static str {
    level.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_builder_basic() {
        let filter = FilterBuilder::new()
            .default_level("debug")
            .allow("my_app")
            .allow_at("my_lib", "trace")
            .build();

        assert!(!format!("{filter:?}").is_empty());
    }

    #[test]
    fn filter_builder_with_directives() {
        let filter = FilterBuilder::new()
            .allow("my_app")
            .directive("hyper=warn")
            .build();

        assert!(!format!("{filter:?}").is_empty());
    }
}
