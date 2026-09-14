#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A unit variant has nowhere to put the member's content: no fields to spread it into, no payload
// to carry the unwrapped value.
#[armonik_macros::oneof("fixture.Choice.choice")]
#[derive(Debug)]
pub enum Choice {
    Text(String),
    Simple(String),
    #[armonik(inlined)]
    Flag,
}
