//! HTTP/2-level transport options.
//!
//! Grouped because their names in the environment already share the `Http2` prefix
//! (`Http2KeepAliveInterval`, `Http2MaxHeaderListSize`, ...): [`serde_with::with_prefix!`]
//! reproduces that prefix from [`Http2ConfigArgs`]'s own field names composed with
//! `#[serde(flatten)]`, so grouping these fields changes no environment variable a deployment
//! already sets.

use std::time::Duration;

#[cfg(feature = "serde")]
use crate::config::text;
use crate::config::{parse_optional_duration, parse_optional_int, ConfigError};

#[cfg(feature = "serde")]
serde_with::with_prefix!(pub(crate) prefix_http2 "Http2");

/// HTTP/2-level transport options, in the string form a caller supplies them in.
///
/// Read from an `Http2`-prefixed variable or JSON key, e.g. [`Self::keep_alive_interval`] is
/// `Http2KeepAliveInterval`: see the module documentation for why.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[non_exhaustive]
pub struct Http2ConfigArgs {
    /// HTTP/2 PING frame interval (e.g. `20s`), defaults to no keepalive. `Http2KeepAliveInterval`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub keep_alive_interval: String,
    /// HTTP/2 PING timeout (e.g. `10s`), defaults to no timeout. `Http2KeepAliveTimeout`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub keep_alive_timeout: String,
    /// Send HTTP/2 keepalive PINGs even when idle, empty for false. `Http2KeepAliveWhileIdle`. See
    /// [`crate::TlsConfigArgs::allow_unsafe_connection`] for the accepted spellings.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub keep_alive_while_idle: String,
    /// HTTP/2 max header list size in bytes, defaults to no limit. `Http2MaxHeaderListSize`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub max_header_list_size: String,
}

/// The resolved form of [`Http2ConfigArgs`].
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct Http2Config {
    /// HTTP/2 PING frame interval, defaults to no keepalive.
    pub keep_alive_interval: Option<Duration>,
    /// HTTP/2 PING timeout, defaults to no timeout.
    pub keep_alive_timeout: Option<Duration>,
    /// Send HTTP/2 keepalive PINGs even when idle, defaults to false.
    pub keep_alive_while_idle: bool,
    /// HTTP/2 max header list size in bytes, defaults to no limit.
    pub max_header_list_size: Option<u32>,
}

impl Http2ConfigArgs {
    pub(crate) fn resolve(self) -> Result<Http2Config, ConfigError> {
        let Self {
            keep_alive_interval,
            keep_alive_timeout,
            keep_alive_while_idle,
            max_header_list_size,
        } = self;

        let keep_alive_interval =
            parse_optional_duration("http2_keep_alive_interval", keep_alive_interval)?;
        let keep_alive_timeout =
            parse_optional_duration("http2_keep_alive_timeout", keep_alive_timeout)?;
        let max_header_list_size =
            parse_optional_int::<u32>("http2_max_header_list_size", max_header_list_size)?;

        Ok(Http2Config {
            keep_alive_interval,
            keep_alive_timeout,
            keep_alive_while_idle: crate::config::parse_bool(
                "http2_keep_alive_while_idle",
                &keep_alive_while_idle,
            )?,
            max_header_list_size,
        })
    }
}
