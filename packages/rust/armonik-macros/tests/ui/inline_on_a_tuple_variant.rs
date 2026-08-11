#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// There is nothing to spread the member's fields into.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Choice", oneof = "choice")]
pub enum Choice {
    Text(String),
    #[armonik(inline)]
    Simple(String),
    #[armonik(present)]
    Flag,
}
