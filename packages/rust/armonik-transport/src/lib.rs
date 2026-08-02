// The usage documentation is the README, included here so that its examples are compiled and cannot
// drift from the code they describe.
#![doc = include_str!("../README.md")]

mod config;
mod connect;
#[cfg(feature = "env")]
mod env;
mod proxy;
mod retry;
mod secret;
mod utils;

pub use config::{ClientConfigArgs, ConfigError, HttpConfig, ProxyConfig, ProxySource};
pub use connect::{connect, https_connector, ConnectionError};
#[cfg(feature = "env")]
pub use env::EnvFieldError;
pub use proxy::ProxyError;
pub use retry::{GrpcStatus, RetryPolicy};
pub use secret::Secret;
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
    // For the `retry!` macro, which sleeps between attempts in the caller's crate.
    pub use tokio;
    pub use tonic;
}
