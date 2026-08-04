//! Transport layer for the ArmoniK Rust client.
//!
//! Configuration parsing, TLS and mTLS: what it takes to turn an endpoint into a connected channel.
//! Depending on this alone leaves protobuf codegen, and the `protoc` a build script would need, out of
//! the build.
//!
//! A caller hands over a [`HttpConfigArgs`], filled in from wherever it keeps its settings.
//! `connect` itself reads neither an environment variable nor a file of options: where the values
//! come from is the business of whoever knows what the deployment looks like. Two exceptions, both
//! opt-in rather than something `connect` does on its own: `ProxySource::System`, the `*_PROXY`
//! convention every HTTP client obeys, and [`HttpConfigArgs::from_env`] (behind the `env`
//! feature), for the common case where a deployment's settings really are one environment variable
//! per option, spelled in `PascalCase` under a prefix of the caller's choosing.

mod config;
mod connect;
#[cfg(feature = "env")]
mod env;
mod http2_config;
mod proxy;
mod proxy_config;
mod secret;
mod tcp_config;
mod tls_config;
mod utils;

pub use config::{ConfigError, HttpConfig, HttpConfigArgs};
pub use connect::{connect, https_connector, ConnectionError};
#[cfg(feature = "env")]
pub use env::EnvFieldError;
pub use http2_config::{Http2Config, Http2ConfigArgs};
pub use proxy::ProxyError;
pub use proxy_config::{HttpProxyConfig, HttpProxyConfigArgs, ProxySource};
pub use secret::Secret;
pub use tcp_config::{TcpConfig, TcpConfigArgs};
pub use tls_config::{HttpTlsConfig, HttpTlsConfigArgs};
// Snafu's context selectors, so a caller in another crate can build the error with the location
// captured at its own call site. Hidden: this is how the error is built, not API to design against.
#[doc(hidden)]
pub use connect::{ConfigSnafu, IoSnafu, TlsSnafu, TransportSnafu};

/// Re-exports of this crate's own dependencies, at the versions it was built with.
///
/// A dependent should take these rather than declare its own requirement for the same crates, so it
/// cannot end up with a `rustls` other than the one the connection was built with.
pub mod reexports {
    pub use hyper;
    pub use hyper_rustls;
    pub use hyper_util;
    pub use rustls;
    #[cfg(feature = "serde")]
    pub use serde;
    pub use tonic;
}
