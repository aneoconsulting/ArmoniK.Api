#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A tuple field has no name to match a proto field by.
#[armonik_macros::message("fixture.Simple")]
#[derive(Debug, Default)]
pub struct Simple(pub String, pub i32);
