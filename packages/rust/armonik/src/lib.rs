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

/// Register a type's proto names into the differential harness's registry. The one place the
/// registration shape is written (the `linkme` slice, the `cfg(test)` gate, the hooks): the macros
/// emit `crate::register!(...)` and the hand-written impl calls it directly, so none restates the
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
    // The two ends of the RPC-coverage check: `service!` declares, `#[armonik_macros::client]`
    // implements, and the test in `differential` asserts the two sets are equal.
    (declared_rpc: $service:literal, $method:literal) => {
        $crate::register!(@rpc DECLARED_RPCS, $service, $method);
    };
    (client_method: $service:literal, $method:literal) => {
        $crate::register!(@rpc CLIENT_METHODS, $service, $method);
    };
    // The service a client impl block is for, so the check knows which ones this build has a client
    // for without reading that off the methods it is checking.
    (client_service: $service:literal) => {
        #[cfg(test)]
        const _: () = {
            #[::linkme::distributed_slice($crate::differential::registrations::CLIENT_SERVICES)]
            static S: &str = $service;
        };
    };

    (@rpc $slice:ident, $service:literal, $method:literal) => {
        #[cfg(test)]
        const _: () = {
            #[::linkme::distributed_slice($crate::differential::registrations::$slice)]
            static R: $crate::differential::registrations::Rpc =
                $crate::differential::registrations::Rpc {
                    service: $service,
                    method: $method,
                };
        };
    };

    // One registration for a proto name with no Rust type of its own.
    (@untyped $proto:literal, $role:ident) => {
        #[cfg(test)]
        const _: () = {
            #[::linkme::distributed_slice($crate::differential::registrations::REGISTRY)]
            static R: $crate::differential::registrations::Registration =
                $crate::differential::registrations::Registration {
                    proto: $proto,
                    role: $crate::differential::registrations::Role::$role,
                };
        };
    };

    // One registration for a real Rust type, with the harness round-trip/projection hooks.
    (@type $proto:literal, $ty:ident) => {
        #[cfg(test)]
        const _: () = {
            #[::linkme::distributed_slice($crate::differential::registrations::REGISTRY)]
            static R: $crate::differential::registrations::Registration =
                $crate::differential::registrations::Registration {
                    proto: $proto,
                    role: $crate::differential::registrations::Role::Message(
                        $crate::differential::registrations::Hooks {
                            type_name: || ::core::any::type_name::<$ty>(),
                            // `encoded_len` is asserted against what is written, on every value the
                            // fuzz and mutation corpora reach. Nothing else checks it: the vector
                            // `encode_to_vec` sizes from just grows when the length under-reports,
                            // so an over- or under-report is invisible to a round trip, and the
                            // derived types get both numbers from one expression while the one
                            // hand-written `Message` impl has two independent matches.
                            roundtrip: |bytes| {
                                let value = <$ty as ::prost::Message>::decode(bytes)?;
                                let bytes = ::prost::Message::encode_to_vec(&value);
                                ::core::assert_eq!(
                                    ::prost::Message::encoded_len(&value),
                                    bytes.len(),
                                    "{}: encoded_len disagrees with what encode wrote",
                                    ::core::any::type_name::<$ty>(),
                                );
                                ::core::result::Result::Ok(bytes)
                            },
                            default_encoding: || ::prost::Message::encode_to_vec(
                                &<$ty as ::core::default::Default>::default(),
                            ),
                            normalize: <$ty as $crate::differential::Normalize>::normalize,
                        },
                    ),
                };
        };
    };
}
pub(crate) use register;

// The object tree carries the ergonomic types; they are re-exported flat below, the only supported
// surface.
mod objects;
pub use objects::*;

// The differential harness and its registry: a unit-test module, so what the suites link is the
// artifact the crate ships.
#[cfg(test)]
mod differential;

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
