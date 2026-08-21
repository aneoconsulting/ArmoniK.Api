#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.Simple` has no `colour`.
#[armonik_macros::message("fixture.Simple")]
#[derive(Debug, Default)]
pub struct Simple {
    pub name: String,
    pub count: i32,
    pub colour: String,
}
