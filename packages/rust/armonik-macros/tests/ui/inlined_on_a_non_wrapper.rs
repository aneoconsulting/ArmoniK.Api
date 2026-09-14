#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

#[armonik_macros::oneof("fixture.Choice.choice")]
#[derive(Debug)]
pub enum Choice {
    Text(String),
    #[armonik(inlined)]
    Simple(String),
    #[armonik(present)]
    Flag,
    Hostile(String),
}
