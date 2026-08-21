#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `present` records that a member was set; it needs a bool or an empty message, not a string.
#[armonik_macros::message("fixture.Choice")]
#[derive(Debug)]
#[armonik(oneof = "choice")]
pub enum Choice {
    #[armonik(present)]
    Text,
    Simple(String),
    #[armonik(present)]
    Flag,
}
