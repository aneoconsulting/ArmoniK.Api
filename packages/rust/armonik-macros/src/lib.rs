//! Internal derive macros for the [`armonik`](https://crates.io/crates/armonik) crate.
//!
//! This crate is an implementation detail of `armonik`: the attribute grammar
//! and the emitted code offer no stability guarantee of their own. It must
//! only be used through the `armonik` crate, which depends on it with an
//! exact version pin.
//!
//! The derives read the protobuf descriptor set compiled by the `armonik`
//! build script (`$OUT_DIR/descriptor.bin`) at expansion time: field tags,
//! wire kinds and cardinalities are taken from the descriptors, and any
//! mismatch between a Rust type and its proto counterpart is a compile
//! error. A fingerprint const-assert is emitted with every expansion so a
//! stale expansion can never survive a descriptor change.

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod attrs;
mod codegen;
mod descriptor;
mod errors;
mod expand;
mod kind;
mod resolve;

/// Derive `prost::Message` for an ArmoniK API type, validated against the
/// protobuf descriptors compiled by the `armonik` build script.
#[proc_macro_derive(Message, attributes(armonik))]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    expand::message(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive the wire representation of a protobuf enum for an ArmoniK API type,
/// validated against the protobuf descriptors compiled by the `armonik` build
/// script.
#[proc_macro_derive(Enum, attributes(armonik))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    expand::enumeration(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
