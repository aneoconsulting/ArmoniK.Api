#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `sfixed32` has no `ProtoField` impl; the codec is keyed by Rust type, so it could not tell one
// from `int32` anyway.
#[armonik_macros::message("fixture.Unsupported")]
#[derive(Debug, Default)]
pub struct Unsupported {
    pub packed: i32,
}
