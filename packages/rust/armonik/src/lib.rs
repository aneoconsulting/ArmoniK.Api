//! Rust bindings for the ArmoniK API
//!
//! Ergonomic Rust structs and enums implementing [`prost::Message`] directly against the ArmoniK
//! protobuf schema, with no generated intermediate representation and no conversion layer, plus the
//! gRPC clients and servers speaking them natively.
//!
//! # Defaults
//!
//! `Default::default()` is the proto zero value for every type, which is what lets decoding seed
//! from it with no special wire semantics: an empty message and `Default::default()` are the same
//! value. A request built as `Ty { field, ..Default::default() }` therefore sends zeros for
//! everything else, including a zero task deadline or a page size of 0. The types that need
//! non-zero values to be useful supply them by name instead:
//! [`TaskOptions::recommended`](TaskOptions::recommended),
//! `<service>::list::Request::recommended()`, and
//! [`Sort::ascending`](Sort::ascending) / [`Sort::descending`](Sort::descending).

// Staleness anchor for the wire-representation derives and the `service!` invocations: `include!`
// puts the generated file in rustc's dep-info, so any descriptor change invalidates the crate;
// every expansion const-asserts against this fingerprint.
mod __schema {
    include!(concat!(env!("OUT_DIR"), "/schema_meta.rs"));
}

pub(crate) mod codec;
pub(crate) mod utils;

/// Register a type's proto names into [`wire::REGISTRY`]. The one place the registration shape is
/// written (the `linkme` slice, the feature gates, the `Diff` hooks): the macros emit
/// `crate::register!(...)` and the two hand-written impls call it directly, so none restates the
/// slice's layout.
///
/// - `message: Ty, "proto.Name", ...`: a Rust type implementing the messages;
/// - `absorbed: "proto.Name", ...`: a message flattened into a parent, no type;
/// - `unexposed: "proto.Name", ...`: a message of an RPC the crate does not expose, no type either,
///   emitted by `service!` from `unexposed(...)`.
macro_rules! register {
    (message: $ty:ident, $($proto:literal),+ $(,)?) => {
        $($crate::register!(@type $proto, $ty);)+
    };
    (absorbed: $($proto:literal),+ $(,)?) => {
        $($crate::register!(@untyped $proto, Absorbed);)+
    };
    (unexposed: $($proto:literal),+ $(,)?) => {
        $($crate::register!(@untyped $proto, Unexposed);)+
    };

    // One registration for a proto name with no Rust type of its own.
    (@untyped $proto:literal, $role:ident) => {
        #[cfg(feature = "_differential")]
        const _: () = {
            #[::linkme::distributed_slice($crate::wire::REGISTRY)]
            static R: $crate::wire::Registration = $crate::wire::Registration {
                proto: $proto,
                role: $crate::wire::Role::$role,
                diff: ::core::option::Option::None,
            };
        };
    };

    // One registration for a real Rust type, with the harness round-trip/projection hooks.
    (@type $proto:literal, $ty:ident) => {
        #[cfg(feature = "_differential")]
        const _: () = {
            #[::linkme::distributed_slice($crate::wire::REGISTRY)]
            static R: $crate::wire::Registration = $crate::wire::Registration {
                proto: $proto,
                role: $crate::wire::Role::Message,
                diff: ::core::option::Option::Some($crate::wire::Diff {
                    roundtrip: |bytes| ::core::result::Result::Ok(
                        ::prost::Message::encode_to_vec(&<$ty as ::prost::Message>::decode(bytes)?),
                    ),
                    default_encoding: || ::prost::Message::encode_to_vec(
                        &<$ty as ::core::default::Default>::default(),
                    ),
                    normalize: <$ty as $crate::differential::Normalize>::normalize,
                }),
            };
        };
    };
}
pub(crate) use register;

// The object tree carries the ergonomic types; they are re-exported flat below, the only supported
// surface.
mod objects;
pub use objects::*;

// The differential harness (test-only). Enabled only through the self dev-dependency, so tests
// always see it and downstream builds never do.
#[cfg(feature = "_differential")]
#[doc(hidden)]
pub mod differential;

// The self-registering type registry, consumed by the differential harness.
#[cfg(feature = "_differential")]
#[doc(hidden)]
pub mod wire;

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

/// The crate's transitive dependencies, re-exported so consumers can name them at the exact
/// versions armonik was built with, whether or not the crate itself still uses them internally.
pub mod reexports {
    pub use bytes;
    // Through `armonik-transport`, which owns these now, so `armonik::reexports::rustls` cannot
    // differ from the `rustls` the connection was built with.
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
    #[cfg(any(feature = "_gen-client", feature = "_gen-server"))]
    pub use tracing;
    #[cfg(any(feature = "_gen-client", feature = "_gen-server"))]
    pub use tracing_futures;
}
