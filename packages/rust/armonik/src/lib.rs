//! Rust bindings for the ArmoniK API

// Staleness anchor for the wire-representation derives: `include!` puts the
// generated file in rustc's dep-info, so any descriptor change invalidates
// the crate; every derive const-asserts against this fingerprint.
mod __schema {
    include!(concat!(env!("OUT_DIR"), "/schema_meta.rs"));
}

#[cfg(any(feature = "_gen-client", feature = "_gen-server"))]
pub(crate) mod api;
#[cfg(feature = "_gen-client")]
pub mod client;
pub(crate) mod codec;
mod objects;
#[cfg(feature = "_gen-server")]
pub mod server;

/// The transport layer: configuration parsing, TLS and the connection itself.
#[cfg(feature = "_gen-client")]
pub use armonik_transport as transport;
#[cfg(feature = "_gen-client")]
pub use client::{Client, ClientConfig};
pub use objects::*;

mod utils;

#[cfg(feature = "_differential")]
#[doc(hidden)]
pub mod differential;

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
