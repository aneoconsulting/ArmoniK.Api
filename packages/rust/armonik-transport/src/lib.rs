//! Transport layer for the ArmoniK Rust client.
//!
//! Configuration parsing, TLS and mTLS: what it takes to turn an endpoint into a connected channel.
//! Split out of [`armonik`](https://docs.rs/armonik) so that a consumer needing only a channel does
//! not also need protobuf codegen and a `protoc` build step.
//!
//! `armonik` re-exports all of this at the paths it always had, so the split breaks nothing for it.

mod config;
mod connect;
mod utils;

pub use config::{ClientConfig, ClientConfigArgs, ConfigError};
pub use connect::{connect, https_connector, ConnectionError};
// Snafu's context selectors, so a caller in another crate can build the error with the location
// captured at its own call site. Hidden: this is how the error is built, not API to design against.
#[doc(hidden)]
pub use connect::{ConfigSnafu, IoSnafu, TlsSnafu, TransportSnafu};
pub use utils::ReadEnvError;

/// Re-exports of this crate's own dependencies, at the versions it was built with.
///
/// `armonik` re-exports these rather than declaring its own requirements for the same crates, so
/// nothing can end up with a `rustls` other than the one the connection was built with.
pub mod reexports {
    pub use hyper;
    pub use hyper_rustls;
    pub use hyper_util;
    pub use rustls;
    #[cfg(feature = "serde")]
    pub use serde;
    pub use tonic;
}
