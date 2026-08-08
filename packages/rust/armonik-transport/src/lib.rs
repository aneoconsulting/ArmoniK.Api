//! Transport layer for the ArmoniK Rust client.
//!
//! Configuration parsing, TLS and mTLS: what it takes to turn an endpoint into a connector requests
//! go out through. Wrapping that connector in an HTTP/2 engine belongs to whoever consumes it.
//! Depending on this alone leaves protobuf codegen, and the `protoc` a build script would need, out of
//! the build.

mod config;
mod config_utils;
mod connector;
#[cfg(feature = "env")]
mod env;
mod http2_config;
mod proxy;
mod retry_config;
mod tcp_config;
mod tls_config;
mod utils;

pub use config::{ConfigError, HttpConfig};
pub use connector::{https_connector, ConnectionError, Connector};
#[cfg(feature = "env")]
pub use env::EnvError;
pub use http2_config::Http2Config;
// `ProxyConnector` is a layer of the stack `Connector` names, so it has to be nameable too.
pub use proxy::{ProxyConfig, ProxyConnector, ProxyError, ProxySource};
pub use retry_config::RetryConfig;
pub use tcp_config::TcpConfig;
pub use tls_config::{Identity, TlsConfig};
// The password's own type, so a caller can build a `ProxyConfig` without declaring a dependency on
// the `secrecy` crate this one happens to use.
pub use secrecy::SecretString;
// Snafu's context selectors, so a caller in another crate can build the error with the location
// captured at its own call site. Hidden: this is how the error is built, not API to design against.
#[doc(hidden)]
pub use connector::{IoSnafu, TlsSnafu};

/// Re-exports of this crate's own dependencies, at the versions it was built with.
///
/// A dependent should take these rather than declare its own requirement for the same crates, so it
/// cannot end up with a `rustls` other than the one the connection was built with.
///
/// Enough of them to build the HTTP/2 engine this crate leaves to its consumer: `h2` and
/// `http_body_util` are here for that alone, and are used nowhere in this crate.
pub mod reexports {
    pub use h2;
    pub use http;
    pub use http_body_util;
    pub use hyper;
    pub use hyper_rustls;
    pub use hyper_util;
    pub use rustls;
    #[cfg(feature = "schema")]
    pub use schemars;
    pub use secrecy;
    #[cfg(feature = "serde")]
    pub use serde;
    // Only the versions match. Which of tokio's modules exist is decided by the features every crate
    // in the build asks for together, so a dependent that needs a runtime still declares tokio for
    // that; taking it from here is what keeps the stream types the connector hands back the same.
    pub use tokio;
}
