#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// What the type stands for is the macro's argument; every type-level key picks a shape this is not.
#[armonik_macros::oneof("fixture.Choice.choice")]
#[derive(Debug)]
#[armonik(transparent)]
pub enum Choice {
    Text(String),
    Simple(String),
    Flag,
}
