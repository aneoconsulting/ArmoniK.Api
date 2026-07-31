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

// The object tree carries the ergonomic types; they are re-exported flat
// below. It is `pub` (not `pub(crate)`) so that the definition paths the
// derives register into `wire::EXTERN_MAP` resolve from the `armonik`
// crate, but `#[doc(hidden)]` so only the flat re-exports are documented.
#[doc(hidden)]
pub mod objects;
pub use objects::*;

#[cfg(feature = "_differential")]
#[doc(hidden)]
pub mod differential;

// Also enabled under `_differential`: the harness reads `wire::ABSORBED` to
// count flattened-away messages as covered.
#[cfg(any(feature = "_extern-map", feature = "_differential"))]
#[doc(hidden)]
pub mod wire;

pub mod reexports {
    pub use bytes;
    pub use prost;
    pub use prost_types;
    #[cfg(feature = "serde")]
    pub use serde;
}
