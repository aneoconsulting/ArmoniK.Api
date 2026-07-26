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

/// Registry of every message type for the differential harness: the derives
/// register each descriptor-validated type here (hand-written impls register
/// themselves), so the harness discovers the proto-to-type mapping instead
/// of maintaining it. Test-only: the `_differential` feature is enabled
/// through the self dev-dependency.
#[cfg(feature = "_differential")]
#[doc(hidden)]
pub mod differential {
    pub struct Entry {
        pub proto: &'static str,
        /// Decode the bytes as the armonik type and re-encode them.
        pub roundtrip: fn(&[u8]) -> Result<Vec<u8>, prost::DecodeError>,
        /// Canonical encoding of the type's `Default`. Doubles as the
        /// zero-default invariant (an empty message must decode to
        /// `Default::default()`) and as the harness's quotient: it is
        /// exactly what the type emits for "nothing", so any field present
        /// here materializes on messages where it is absent.
        pub default_encoding: fn() -> Vec<u8>,
        /// Members carried by `#[armonik(present)]` bool markers: only
        /// their presence survives (an explicit `false` reads as set).
        pub bool_markers: &'static [&'static str],
        /// Transparent wrapper (chain) enums: zero, absent and
        /// present-but-empty carry no information at any depth.
        pub wrapper_chain: bool,
    }

    #[linkme::distributed_slice]
    pub static REGISTRY: [Entry];
}

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
