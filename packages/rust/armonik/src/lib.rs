//! Rust bindings for the ArmoniK API

pub mod api;
#[cfg(feature = "_gen-client")]
pub mod client;
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

pub mod reexports {
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
