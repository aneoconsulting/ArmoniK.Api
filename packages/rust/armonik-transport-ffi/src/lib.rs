//! C ABI over the ArmoniK Rust transport, for host applications that cannot link Rust directly.
//!
//! # Scope
//!
//! This crate speaks HTTP, not gRPC. A request is opened with a set of headers, bytes are streamed
//! in both directions, and headers, data and trailers come back. There is no notion here of a call
//! shape, of a `grpc-status`, of message framing or of retry: those belong to whatever gRPC stack
//! the host application already has, and this crate is the transport underneath it. Configuration
//! parsing, TLS, mTLS and proxy handling all live in
//! [`armonik_transport`](https://docs.rs/armonik-transport); this crate adapts that connection layer
//! to a C ABI.
//!
//! # The contract
//!
//! `include/armonik_transport_ffi.h` is the whole of it, generated from the items below and
//! committed so that a change shows up in review rather than only in a compiled library. Its
//! preamble carries what a signature cannot: the sign of the result codes, which buffers are owned
//! and which borrowed, the key/value encoding, the rules the event callback is delivered under, and
//! what may and may not change from one revision of the ABI to the next.
//!
//! Two of those rules are worth repeating where the code is. Nothing that crosses the boundary
//! promises an ordering between two independent events; a test asserts the set of acceptable
//! outcomes rather than a single one. And a handle is reference-counted: `_release` gives back one
//! reference, so a call already under way when another thread releases the handle finishes normally.
//!
//! # Safety discipline
//!
//! - Every entry point runs its body through one of the `guard` module's wrappers, so a panic can
//!   never cross the ABI boundary.
//! - Every allocation handed to the caller travels as an [`ak_bytes`] and is given up through
//!   [`ak_bytes_release`] exactly once. Only a synchronous out-parameter produces one.
//! - Every opaque handle lives in the `handle` module's registry, which owns it and lends counted
//!   references for the duration of a call.

#![deny(missing_docs)]
// The FFI type names mirror the C header, e.g. `ak_bytes`.
#![allow(non_camel_case_types)]

// `dead_code` asks whether an item is reachable from this crate's public Rust API. That is not this
// crate's contract: what it offers is the `extern "C"` surface, and these modules are the primitives
// behind it, each covered by its own tests.
#[allow(dead_code)]
mod blob;
mod client;
#[allow(dead_code)]
mod error;
#[allow(dead_code)]
mod guard;
#[allow(dead_code)]
mod handle;
#[allow(dead_code)]
mod rate_limit;
#[cfg(test)]
mod test_support;

pub mod status;
// Public, and hidden, for exactly one reason: `runtime::alive_tasks` lets leak assertions check that
// the runtime comes back to rest. Nothing in it is `extern "C"`, so none of it reaches the generated
// header, and so none of it is part of the ABI.
#[doc(hidden)]
pub mod runtime;

pub use client::{ak_client, ak_client_create, ak_client_release};
pub use error::{ak_bytes, ak_bytes_in, ak_bytes_release};

/// The revision of this ABI that this library implements.
///
/// A caller compiles this value in and compares it against what [`ak_abi_version`] answers.
pub const AK_ABI_VERSION: i32 = 1;

/// The revision of this ABI that the loaded library implements.
///
/// Queryable rather than only compiled in, because a host process loads one native module and every
/// add-in in it shares whichever was loaded first: an add-in that did not bring its own has to be
/// able to find out what it got. Asking turns a mismatch into a diagnosis, where reaching for an
/// entry point that is not there surfaces as an `EntryPointNotFoundException` from somewhere
/// unrelated.
///
/// Compare against `AK_ABI_VERSION`, the value this library was compiled with.
#[no_mangle]
pub extern "C" fn ak_abi_version() -> i32 {
    // No guard: returning a constant cannot unwind, and this is the one call a caller makes before
    // it trusts anything else here.
    AK_ABI_VERSION
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_reported_version_is_the_compiled_one() {
        assert_eq!(super::ak_abi_version(), super::AK_ABI_VERSION);
    }
}
