//! TCP-level socket options.
//!
//! Grouped because their names in the environment already share the `Tcp` prefix
//! (`TcpKeepalive`, `TcpKeepaliveInterval`, ...): [`serde_with::with_prefix!`] reproduces that
//! prefix from [`TcpConfigArgs`]'s own field names composed with `#[serde(flatten)]`, so grouping
//! these fields changes no environment variable a deployment already sets.

use std::time::Duration;

use snafu::ResultExt;

#[cfg(feature = "serde")]
use crate::config::text;
use crate::config::{ConfigError, InvalidDurationSnafu, InvalidIntegerSnafu};

#[cfg(feature = "serde")]
serde_with::with_prefix!(pub(crate) prefix_tcp "Tcp");

/// TCP-level socket options, in the string form a caller supplies them in.
///
/// Read from a `Tcp`-prefixed variable or JSON key, e.g. [`Self::keepalive`] is `TcpKeepalive`: see
/// the module documentation for why.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase", default))]
#[non_exhaustive]
pub struct TcpConfigArgs {
    /// TCP keepalive duration (e.g. `30s`), defaults to no keepalive. `TcpKeepalive`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub keepalive: String,
    /// Interval between TCP keepalive probes (e.g. `5s`), defaults to OS default.
    /// `TcpKeepaliveInterval`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub keepalive_interval: String,
    /// Number of TCP keepalive retries, defaults to OS default. `TcpKeepaliveRetries`.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub keepalive_retries: String,
    /// Enable Nagle's algorithm (disable TCP_NODELAY), empty for false. `TcpNagleAlgorithm`. See
    /// [`crate::HttpTlsConfigArgs::allow_unsafe_connection`] for the accepted spellings.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "text"))]
    pub nagle_algorithm: String,
}

/// The resolved form of [`TcpConfigArgs`].
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct TcpConfig {
    /// TCP keepalive duration, defaults to no keepalive.
    pub keepalive: Option<Duration>,
    /// Interval between TCP keepalive probes, defaults to OS default.
    pub keepalive_interval: Option<Duration>,
    /// Number of TCP keepalive retries, defaults to OS default.
    pub keepalive_retries: Option<u32>,
    /// Enable Nagle's algorithm (disable TCP_NODELAY), defaults to false.
    pub nagle_algorithm: bool,
}

impl TcpConfigArgs {
    pub(crate) fn resolve(self) -> Result<TcpConfig, ConfigError> {
        let Self {
            keepalive,
            keepalive_interval,
            keepalive_retries,
            nagle_algorithm,
        } = self;

        let keepalive = if keepalive.is_empty() {
            None
        } else {
            Some(
                keepalive
                    .parse::<humantime::Duration>()
                    .context(InvalidDurationSnafu {
                        option: "tcp_keepalive",
                        value: keepalive,
                    })?
                    .into(),
            )
        };

        let keepalive_interval = if keepalive_interval.is_empty() {
            None
        } else {
            Some(
                keepalive_interval
                    .parse::<humantime::Duration>()
                    .context(InvalidDurationSnafu {
                        option: "tcp_keepalive_interval",
                        value: keepalive_interval,
                    })?
                    .into(),
            )
        };

        let keepalive_retries = if keepalive_retries.is_empty() {
            None
        } else {
            Some(
                keepalive_retries
                    .parse::<u32>()
                    .context(InvalidIntegerSnafu {
                        option: "tcp_keepalive_retries",
                        value: keepalive_retries,
                    })?,
            )
        };

        Ok(TcpConfig {
            keepalive,
            keepalive_interval,
            keepalive_retries,
            nagle_algorithm: crate::config::parse_bool("tcp_nagle_algorithm", &nagle_algorithm)?,
        })
    }
}
