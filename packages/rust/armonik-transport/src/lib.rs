//! Transport layer for the ArmoniK Rust client.
//!
//! Configuration parsing, TLS and mTLS: what it takes to turn an endpoint into a connected channel.
//! Depending on this alone leaves protobuf codegen, and the `protoc` a build script would need, out of
//! the build.

mod config;
mod connect;
mod proxy;
mod utils;

pub use config::{ConfigError, HttpConfig, HttpConfigArgs};
pub use connect::{connect, https_connector, ConnectionError};
pub use proxy::{ProxyConfig, ProxyError, ProxySource};
// The password's own type, so a caller can build a `ProxyConfig` without declaring a dependency on
// the `secrecy` crate this one happens to use.
pub use secrecy::SecretString;
// Snafu's context selectors, so a caller in another crate can build the error with the location
// captured at its own call site. Hidden: this is how the error is built, not API to design against.
#[doc(hidden)]
pub use connect::{ConfigSnafu, IoSnafu, TlsSnafu, TransportSnafu};
pub use utils::ReadEnvError;

/// Re-exports of this crate's own dependencies, at the versions it was built with.
///
/// A dependent should take these rather than declare its own requirement for the same crates, so it
/// cannot end up with a `rustls` other than the one the connection was built with.
pub mod reexports {
    pub use hyper;
    pub use hyper_rustls;
    pub use hyper_util;
    pub use rustls;
    pub use secrecy;
    #[cfg(feature = "serde")]
    pub use serde;
    pub use tonic;
}
