//! ArmoniK API message types.
//!
//! Ergonomic Rust structs and enums that implement [`prost::Message`]
//! directly against the ArmoniK protobuf schema — no generated intermediate
//! representation, no conversion layer. Depend on this crate to speak the
//! ArmoniK wire types without the client/server stubs and their
//! tonic/hyper/rustls graph; the [`armonik`] crate re-exports everything here
//! and adds those stubs.
//!
//! [`armonik`]: https://docs.rs/armonik

// Staleness anchor for the wire-representation derives: `include!` puts the
// generated file in rustc's dep-info, so any descriptor change invalidates
// the crate; every derive const-asserts against this fingerprint.
mod __schema {
    include!(concat!(env!("OUT_DIR"), "/schema_meta.rs"));
}

pub(crate) mod codec;
pub(crate) mod utils;

/// Register a type's proto name(s) into [`wire::REGISTRY`]. The single place
/// the registration shape (the `linkme` slice, the feature gates, the `Diff`
/// hooks) is written — the derives emit `crate::register!(...)` and the two
/// hand-written impls call it directly, so neither restates the slice's layout.
///
/// - `message: Ty, "proto.Name", ...` — a Rust type implementing the message(s);
/// - `replace: Ty, message = "...", service = "...", method = "...", input|output,
///   target = "..."` — a per-RPC substitution (see [`wire::Replacement`]);
/// - `absorbed: "proto.Name", ...` — a message flattened into a parent (no type).
macro_rules! register {
    (message: $ty:ident, $($proto:literal),+ $(,)?) => {
        $($crate::register!(@type $proto, $crate::wire::Role::Message {
            rust_path: ::core::concat!(::core::module_path!(), "::", ::core::stringify!($ty)),
        }, $ty);)+
    };
    (replace: $ty:ident, message = $proto:literal, service = $service:literal,
        method = $method:literal, $direction:ident, target = $target:literal $(,)?) => {
        $crate::register!(@type $proto, $crate::wire::Role::Replace($crate::wire::Replacement {
            service: $service,
            method: $method,
            direction: $crate::register!(@dir $direction),
            message: $proto,
            target: $target,
            rust_path: ::core::concat!(::core::module_path!(), "::", ::core::stringify!($ty)),
        }), $ty);
    };
    (absorbed: $($proto:literal),+ $(,)?) => {
        $(
            #[cfg(feature = "_registry")]
            const _: () = {
                #[::linkme::distributed_slice($crate::wire::REGISTRY)]
                static R: $crate::wire::Registration = $crate::wire::Registration {
                    proto: $proto,
                    role: $crate::wire::Role::Absorbed,
                    #[cfg(feature = "_differential")]
                    diff: ::core::option::Option::None,
                };
            };
        )+
    };

    (@dir input) => { $crate::wire::Direction::Input };
    (@dir output) => { $crate::wire::Direction::Output };

    // One registration for a real Rust type: its role, plus (under
    // `_differential`) the harness round-trip/projection hooks.
    (@type $proto:literal, $role:expr, $ty:ident) => {
        #[cfg(feature = "_registry")]
        const _: () = {
            #[::linkme::distributed_slice($crate::wire::REGISTRY)]
            static R: $crate::wire::Registration = $crate::wire::Registration {
                proto: $proto,
                role: $role,
                #[cfg(feature = "_differential")]
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

// The object tree carries the ergonomic types; they are re-exported flat
// below. It is `pub` (not `pub(crate)`) so that the definition paths the
// derives register into `wire::REGISTRY` resolve from the `armonik`
// crate, but `#[doc(hidden)]` so only the flat re-exports are documented.
#[doc(hidden)]
pub mod objects;
pub use objects::*;

// The differential harness (test-only). Extends `_registry`, adding the
// `prost-reflect`-typed projection hooks; enabled through the self dev-dependency.
#[cfg(feature = "_differential")]
#[doc(hidden)]
pub mod differential;

// The self-registering type registry, read by `armonik`'s build script.
// `_differential` implies `_registry`, so the module is present whenever either
// consumer is active.
#[cfg(feature = "_registry")]
#[doc(hidden)]
pub mod wire;

pub mod reexports {
    pub use bytes;
    pub use prost;
    pub use prost_types;
    #[cfg(feature = "serde")]
    pub use serde;
}
