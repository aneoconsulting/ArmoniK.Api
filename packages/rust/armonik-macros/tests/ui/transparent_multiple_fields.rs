#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `transparent` delegates the whole impl to one field, so there must be exactly one.
#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(transparent, message = "fixture.Simple")]
pub struct Simple {
    pub name: String,
    pub count: i32,
}
