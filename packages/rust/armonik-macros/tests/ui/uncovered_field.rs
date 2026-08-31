#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.Simple` declares `count` (tag 2), which no field covers.
#[armonik_macros::message("fixture.Simple")]
#[derive(Debug, Default)]
pub struct Simple {
    pub name: String,
}
