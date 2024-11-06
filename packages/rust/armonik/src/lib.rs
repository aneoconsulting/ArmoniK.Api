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
    #[cfg(feature = "_gen-client")]
    pub use hyper;
    #[cfg(feature = "_gen-client")]
    pub use hyper_rustls;
    pub use prost;
    pub use prost_types;
    #[cfg(feature = "_gen-client")]
    pub use rustls;
    #[cfg(feature = "_gen-server")]
    pub use tokio;
    #[cfg(feature = "_gen-server")]
    pub use tokio_util;
    pub use tonic;
    pub use tonic::async_trait;
    pub use tonic::codegen::http;
    pub use tonic::codegen::tokio_stream;
}
