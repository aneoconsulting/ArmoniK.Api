#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `transparent` delegates the whole impl to one field, so there must be exactly one.
#[armonik_macros::message("fixture.Simple")]
#[derive(Debug, Default)]
#[armonik(transparent)]
pub struct Simple {
    pub name: String,
    pub count: i32,
}
