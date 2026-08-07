//! TCP-level socket options.
//!
//! A unit of fields, not a naming scheme: the embedding composes them with `#[serde(flatten)]`
//! under a prefix of its own, so the same unit serves however many embeddings read TCP options,
//! and grouping these fields changes no environment variable a deployment already sets.

use std::time::Duration;

/// TCP-level socket options.
///
/// Each field names one option; the full name a source spells is the embedding's prefix followed
/// by that name, so no field doc spells it: the same field is a different option under a different
/// prefix. A schema generated for this type on its own describes the unprefixed names it declares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(transform = crate::config_utils::strip_defaults)
)]
#[non_exhaustive]
pub struct TcpConfig {
    /// TCP keepalive duration (e.g. `30s`), defaults to no keepalive.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_duration")
    )]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keepalive: Option<Duration>,
    /// Interval between TCP keepalive probes (e.g. `5s`), defaults to OS default.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_duration")
    )]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keepalive_interval: Option<Duration>,
    /// Number of TCP keepalive retries, defaults to OS default.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_u32")
    )]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keepalive_retries: Option<u32>,
    /// Enable Nagle's algorithm (disable TCP_NODELAY), defaults to false. See
    /// [`crate::TlsConfig::allow_unsafe_connection`] for the accepted spellings.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::bool_option")
    )]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub nagle_algorithm: bool,
}
