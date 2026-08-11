#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(message = "fixture.Nope")]
pub struct Simple {
    pub name: String,
}
