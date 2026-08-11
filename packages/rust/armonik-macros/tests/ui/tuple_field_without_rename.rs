#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A tuple field has no name to match a proto field by.
#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(message = "fixture.Simple")]
pub struct Simple(pub String, pub i32);
