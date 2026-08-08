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

#![deny(missing_docs)]
// The FFI type names mirror the C header, e.g. `ak_status`.
#![allow(non_camel_case_types)]

// `dead_code` asks whether an item is reachable from this crate's public Rust API. That is not this
// crate's contract: what it offers is the `extern "C"` surface, and this module is a primitive
// behind it, covered by its own tests.
#[allow(dead_code)]
mod guard;
#[cfg(test)]
mod test_support;

pub mod status;
