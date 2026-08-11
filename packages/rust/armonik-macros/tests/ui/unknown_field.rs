#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.Simple` has no `colour`.
#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(message = "fixture.Simple")]
pub struct Simple {
    pub name: String,
    pub count: i32,
    pub colour: String,
}
