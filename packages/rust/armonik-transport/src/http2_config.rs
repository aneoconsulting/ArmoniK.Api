//! HTTP/2-level transport options.
//!
//! A unit of fields, not a naming scheme: the embedding composes them with `#[serde(flatten)]`
//! under a prefix of its own, so the same unit serves however many embeddings read HTTP/2 options,
//! and grouping these fields changes no environment variable a deployment already sets.

use std::time::Duration;

/// HTTP/2-level transport options.
///
/// Each field names one option; the full name a source spells is the embedding's prefix followed
/// by that name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[non_exhaustive]
pub struct Http2Config {
    /// HTTP/2 PING frame interval (e.g. `20s`), defaults to no keepalive. `KeepAliveInterval`.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_duration")
    )]
    pub keep_alive_interval: Option<Duration>,
    /// HTTP/2 PING timeout (e.g. `10s`), defaults to no timeout. `KeepAliveTimeout`.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_duration")
    )]
    pub keep_alive_timeout: Option<Duration>,
    /// Send HTTP/2 keepalive PINGs even when idle, defaults to false. `KeepAliveWhileIdle`.
    /// See [`crate::TlsConfig::allow_unsafe_connection`] for the accepted spellings.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::bool_option")
    )]
    pub keep_alive_while_idle: bool,
    /// HTTP/2 max header list size in bytes, defaults to no limit. `MaxHeaderListSize`.
    #[cfg_attr(
        feature = "serde",
        serde(deserialize_with = "crate::config_utils::optional_u32")
    )]
    pub max_header_list_size: Option<u32>,
}
