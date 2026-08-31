#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

#[armonik_macros::oneof("fixture.Choice.choice")]
#[derive(Debug)]
pub enum Choice {
    #[armonik(inlined)]
    Text(String),
    Simple(String),
    #[armonik(present)]
    Flag,
    Hostile(String),
}
