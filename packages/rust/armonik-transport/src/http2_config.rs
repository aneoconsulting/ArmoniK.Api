//! HTTP/2-level transport options.
//!
//! Grouped because their names in the environment already share the `Http2` prefix
//! (`Http2KeepAliveInterval`, `Http2MaxHeaderListSize`, ...): [`serde_with::with_prefix!`]
//! reproduces that prefix from [`Http2Config`]'s own field names composed with
//! `#[serde(flatten)]`, so grouping these fields changes no environment variable a deployment
//! already sets.

use std::time::Duration;

#[cfg(feature = "serde")]
serde_with::with_prefix!(pub(crate) prefix_http2 "Http2");

#[cfg(feature = "schema")]
crate::config::make_schemars_prefix!(http2_schema, Http2Config, "Http2");

/// HTTP/2-level transport options.
///
/// The `Http2` prefix belongs to the embedding, not to these names: flattened into
/// [`crate::HttpConfig`], [`Self::keep_alive_interval`] is read from `Http2KeepAliveInterval`, and
/// a schema generated for this type on its own describes the unprefixed names it declares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[cfg_attr(
    feature = "schema",
    derive(schemars::JsonSchema),
    schemars(transform = crate::config::strip_defaults)
)]
#[non_exhaustive]
pub struct Http2Config {
    /// HTTP/2 PING frame interval (e.g. `20s`), defaults to no keepalive. `Http2KeepAliveInterval`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "keep_alive_interval"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keep_alive_interval: Option<Duration>,
    /// HTTP/2 PING timeout (e.g. `10s`), defaults to no timeout. `Http2KeepAliveTimeout`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "keep_alive_timeout"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keep_alive_timeout: Option<Duration>,
    /// Send HTTP/2 keepalive PINGs even when idle, defaults to false. `Http2KeepAliveWhileIdle`.
    /// See [`crate::TlsConfig::allow_unsafe_connection`] for the accepted spellings.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "keep_alive_while_idle"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub keep_alive_while_idle: bool,
    /// HTTP/2 max header list size in bytes, defaults to no limit. `Http2MaxHeaderListSize`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "max_header_list_size"))]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub max_header_list_size: Option<u32>,
}

/// [`crate::config::optional_duration`], naming this field's own option.
#[cfg(feature = "serde")]
fn keep_alive_interval<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    crate::config::optional_duration("Http2KeepAliveInterval", deserializer)
}

/// [`crate::config::optional_duration`], naming this field's own option.
#[cfg(feature = "serde")]
fn keep_alive_timeout<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    crate::config::optional_duration("Http2KeepAliveTimeout", deserializer)
}

/// [`crate::config::optional_u32`], naming this field's own option.
#[cfg(feature = "serde")]
fn max_header_list_size<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    crate::config::optional_u32("Http2MaxHeaderListSize", deserializer)
}

/// [`crate::config::bool_option`], naming this field's own option: a flattened `serde` source
/// buffers values before handing them over, so the name is no longer available by the time the
/// value is read.
#[cfg(feature = "serde")]
fn keep_alive_while_idle<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<bool, D::Error> {
    crate::config::bool_option("Http2KeepAliveWhileIdle", deserializer)
}
