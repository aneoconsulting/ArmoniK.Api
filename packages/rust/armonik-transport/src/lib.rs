//! Transport layer for the ArmoniK Rust client.
//!
//! This is "how do I get a channel that speaks to an ArmoniK endpoint": configuration parsing, TLS and
//! mTLS, and the timeout, keepalive and identity settings that go with them — factored out of the
//! [`armonik`](https://docs.rs/armonik) crate so it can be depended on without pulling in protobuf
//! codegen or any knowledge of ArmoniK's services.
//!
//! `armonik` re-exports everything here at the paths it always had (`armonik::ClientConfig`,
//! `armonik::client::ConfigError`, ...), so the split is not a breaking change for it. It exists so
//! that a consumer needing only a configured, connected channel can have exactly that — in particular
//! without a `protoc`/`tonic-prost-build` build step, since nothing here touches a generated proto
//! type.

mod config;
mod connect;
mod utils;

pub use config::{ClientConfig, ClientConfigArgs, ConfigError};
pub use connect::{connect, https_connector, ConnectionError};
// The context selectors snafu generates for `ConnectionError`, so a caller in another crate can build one
// with `.context(ConfigSnafu {})` and have the location captured at its own call site rather than in here.
// Hidden from the docs: they are how the error is built, not API to design against.
#[doc(hidden)]
pub use connect::{ConfigSnafu, IoSnafu, TlsSnafu, TransportSnafu};
pub use utils::ReadEnvError;

/// Re-exports of this crate's own dependencies, at the versions it was built with.
///
/// `armonik` re-exports these in turn rather than declaring its own version requirements for the same
/// crates, so nothing in the workspace can resolve to two incompatible copies of `rustls` (or of
/// `tonic`, or of `hyper`) in one build.
pub mod reexports {
    pub use hyper;
    pub use hyper_rustls;
    pub use hyper_util;
    pub use rustls;
    #[cfg(feature = "serde")]
    pub use serde;
    pub use tonic;
}
