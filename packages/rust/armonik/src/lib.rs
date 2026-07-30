//! Rust bindings for the ArmoniK API

pub mod api;
#[cfg(feature = "_gen-client")]
pub mod client;
mod objects;
#[cfg(feature = "_gen-server")]
pub mod server;

#[cfg(feature = "_gen-client")]
pub use client::{Client, ClientConfig};
pub use objects::*;

mod utils;

pub mod reexports {
    // Through `armonik-transport`, which owns these now, so that a downstream user of
    // `armonik::reexports::rustls` cannot end up with a different `rustls` from the one the connection
    // was actually built with.
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
