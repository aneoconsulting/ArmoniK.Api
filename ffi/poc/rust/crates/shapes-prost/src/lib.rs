//! The `prost` arm's types: prost-build's generated structs for
//! `ffi/schema/generated/shapes.proto`, and nothing else.

pub mod shapes {
    include!(concat!(env!("OUT_DIR"), "/armonik.ffi.shapes.v1.rs"));
}

/// The `FileDescriptorSet` protox compiled, so a test can drive a second protobuf
/// implementation over the same descriptor.
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));
