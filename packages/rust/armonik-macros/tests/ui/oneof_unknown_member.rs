#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.Choice.choice` has no member `picture`.
#[armonik_macros::oneof("fixture.Choice.choice")]
#[derive(Debug)]
pub enum Choice {
    Text(String),
    Picture(String),
}
