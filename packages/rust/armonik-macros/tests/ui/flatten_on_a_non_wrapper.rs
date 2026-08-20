#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Choice", oneof = "choice")]
pub enum Choice {
    Text(String),
    #[armonik(flatten)]
    Simple(String),
    #[armonik(present)]
    Flag,
    Hostile(String),
}
