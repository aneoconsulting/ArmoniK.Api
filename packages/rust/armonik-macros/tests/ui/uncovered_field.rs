#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.Simple` declares `count` (tag 2), which no field covers.
#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(message = "fixture.Simple")]
pub struct Simple {
    pub name: String,
}
