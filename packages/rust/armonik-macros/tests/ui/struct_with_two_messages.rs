#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A unified type stands for several identical protos; the struct side of that never had a user.
#[armonik_macros::message("fixture.Simple", "fixture.Typed")]
#[derive(Debug, Default)]
pub struct Simple {
    pub name: String,
    pub count: i32,
}
