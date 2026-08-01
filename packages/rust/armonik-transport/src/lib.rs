//! Transport layer for the ArmoniK Rust client.
//!
//! Configuration parsing, TLS and mTLS: what it takes to turn an endpoint into a connected channel.
//! Depending on this alone leaves protobuf codegen, and the `protoc` a build script would need, out of
//! the build.
//!
//! A caller hands over a [`ClientConfigArgs`], filled in from wherever it keeps its settings. Nothing
//! here reads an environment variable or a file of options: where the values come from is the business
//! of whoever knows what the deployment looks like. The exception is `ProxySource::System`, which is
//! the `*_PROXY` convention every HTTP client obeys rather than a setting of ArmoniK's.

mod config;
mod connect;
mod proxy;
mod secret;
mod tcp;

pub use config::{ClientConfig, ClientConfigArgs, ConfigError, ProxyConfig, ProxySource};
pub use connect::{connect, https_connector, ConnectionError};
pub use proxy::ProxyError;
pub use secret::Secret;

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
