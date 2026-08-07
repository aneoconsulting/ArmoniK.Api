//! TCP-level socket options.
//!
//! Grouped because their names in the environment already share the `Tcp` prefix
//! (`TcpKeepalive`, `TcpKeepaliveInterval`, ...): [`serde_with::with_prefix!`] reproduces that
//! prefix from [`TcpConfig`]'s own field names composed with `#[serde(flatten)]`, so grouping
//! these fields changes no environment variable a deployment already sets.

use std::time::Duration;

#[cfg(feature = "serde")]
serde_with::with_prefix!(pub(crate) prefix_tcp "Tcp");

#[cfg(feature = "schema")]
crate::config::make_schemars_prefix!(tcp_schema, TcpConfig, "Tcp");

/// TCP-level socket options.
///
/// The `Tcp` prefix belongs to the embedding, not to these names: flattened into
/// [`crate::HttpConfig`], [`Self::keepalive`] is read from `TcpKeepalive`, and a schema generated
/// for this type on its own describes the unprefixed names it declares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(transform = crate::config::strip_defaults)
)]
#[non_exhaustive]
pub struct TcpConfig {
    /// TCP keepalive duration (e.g. `30s`), defaults to no keepalive. `TcpKeepalive`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "keepalive"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keepalive: Option<Duration>,
    /// Interval between TCP keepalive probes (e.g. `5s`), defaults to OS default.
    /// `TcpKeepaliveInterval`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "keepalive_interval"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keepalive_interval: Option<Duration>,
    /// Number of TCP keepalive retries, defaults to OS default. `TcpKeepaliveRetries`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "keepalive_retries"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keepalive_retries: Option<u32>,
    /// Enable Nagle's algorithm (disable TCP_NODELAY), defaults to false. `TcpNagleAlgorithm`. See
    /// [`crate::TlsConfig::allow_unsafe_connection`] for the accepted spellings.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "nagle_algorithm"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub nagle_algorithm: bool,
}

/// [`crate::config::bool_option`], naming this field's own option: a flattened `serde` source
/// buffers values before handing them over, so the name is no longer available by the time the
/// value is read.
#[cfg(feature = "serde")]
fn nagle_algorithm<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    crate::config::bool_option("TcpNagleAlgorithm", deserializer)
}

/// [`crate::config::optional_duration`], naming this field's own option.
#[cfg(feature = "serde")]
fn keepalive<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    crate::config::optional_duration("TcpKeepalive", deserializer)
}

/// [`crate::config::optional_duration`], naming this field's own option.
#[cfg(feature = "serde")]
fn keepalive_interval<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    crate::config::optional_duration("TcpKeepaliveInterval", deserializer)
}

/// [`crate::config::optional_u32`], naming this field's own option.
#[cfg(feature = "serde")]
fn keepalive_retries<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    crate::config::optional_u32("TcpKeepaliveRetries", deserializer)
}
