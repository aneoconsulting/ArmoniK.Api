#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// An enum stands for the oneof of the message it names: there is no single field to delegate to.
#[armonik_macros::message("fixture.Choice")]
#[derive(Debug)]
#[armonik(transparent)]
pub enum Choice {
    Text(String),
    Simple(String),
    Flag,
}
