//! Internal derive macros for the [`armonik`](https://crates.io/crates/armonik) crate.
//!
//! This crate is an implementation detail of `armonik`: the attribute grammar
//! and the emitted code offer no stability guarantee of their own. It must
//! only be used through the `armonik` crate, which depends on it with an
//! exact version pin.

use proc_macro::TokenStream;

/// Derive `prost::Message` for an ArmoniK API type, validated against the
/// protobuf descriptors compiled by the `armonik` build script.
#[proc_macro_derive(Message, attributes(armonik))]
pub fn derive_message(_input: TokenStream) -> TokenStream {
    "compile_error!(\"armonik_macros::Message is not implemented yet\");"
        .parse()
        .unwrap()
}

/// Derive the wire representation of a protobuf enum for an ArmoniK API type,
/// validated against the protobuf descriptors compiled by the `armonik` build
/// script.
#[proc_macro_derive(Enum, attributes(armonik))]
pub fn derive_enum(_input: TokenStream) -> TokenStream {
    "compile_error!(\"armonik_macros::Enum is not implemented yet\");"
        .parse()
        .unwrap()
}
