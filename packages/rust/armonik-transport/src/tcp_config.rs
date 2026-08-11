//! TCP-level socket options.
//!
//! A unit of fields, not a naming scheme: the embedding composes them with `#[serde(flatten)]`
//! under a prefix of its own, so the same unit serves however many embeddings read TCP options,
//! and grouping these fields changes no environment variable a deployment already sets.

use std::time::Duration;

/// TCP-level socket options.
///
/// Each field names one option; the full name a source spells is the embedding's prefix followed
/// by that name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[non_exhaustive]
pub struct TcpConfig {
    /// TCP keepalive duration (e.g. `30s`), defaults to no keepalive. `Keepalive`.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_duration")
    )]
    pub keepalive: Option<Duration>,
    /// Interval between TCP keepalive probes (e.g. `5s`), defaults to OS default.
    /// `KeepaliveInterval`.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_duration")
    )]
    pub keepalive_interval: Option<Duration>,
    /// Number of TCP keepalive retries, defaults to OS default. `KeepaliveRetries`.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_u32")
    )]
    pub keepalive_retries: Option<u32>,
    /// Enable Nagle's algorithm (disable TCP_NODELAY), defaults to false. `NagleAlgorithm`. See
    /// [`crate::TlsConfig::allow_unsafe_connection`] for the accepted spellings.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::bool_option")
    )]
    pub nagle_algorithm: bool,
}
