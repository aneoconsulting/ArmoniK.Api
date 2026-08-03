//! Rust bindings for the ArmoniK API

// The message types, codec, and derives live in `armonik-types`; re-export
// its whole surface so `armonik::applications::Raw`, `armonik::TaskOptions`,
// etc. keep resolving. This crate adds the tonic client/server stubs on top.
pub use armonik_types::*;

#[cfg(any(feature = "_gen-client", feature = "_gen-server"))]
pub(crate) mod stubs;

pub mod rpc;

#[cfg(feature = "_gen-client")]
pub mod client;
#[cfg(feature = "_gen-server")]
pub mod server;

/// The transport layer: configuration parsing, TLS and the connection itself.
#[cfg(feature = "_gen-client")]
pub use armonik_transport as transport;
#[cfg(feature = "_gen-client")]
pub use client::{Client, ClientConfig};

mod utils;

pub mod reexports {
    pub use bytes;
    // Through `armonik-transport`, which owns these now, so `armonik::reexports::rustls` cannot differ
    // from the `rustls` the connection was built with.
    #[cfg(feature = "_gen-client")]
    pub use armonik_transport::reexports::{hyper, hyper_rustls, rustls};
    pub use prost;
    pub use prost_types;
    #[cfg(feature = "serde")]
    pub use serde;
    #[cfg(feature = "_gen-server")]
    pub use tokio;
    pub use tonic;
    pub use tonic::async_trait;
    pub use tonic::codegen::http;
    pub use tonic::codegen::tokio_stream;
    pub use tracing;
    pub use tracing_futures;
}
