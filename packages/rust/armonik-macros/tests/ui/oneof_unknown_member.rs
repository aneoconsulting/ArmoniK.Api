#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.Choice.choice` has no member `picture`.
#[armonik_macros::message("fixture.Choice")]
#[derive(Debug)]
#[armonik(oneof = "choice")]
pub enum Choice {
    Text(String),
    Picture(String),
}
