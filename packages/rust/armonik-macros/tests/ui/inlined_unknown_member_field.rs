#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.Simple` has no `nope`.
#[armonik_macros::message("fixture.Choice")]
#[derive(Debug)]
#[armonik(oneof = "choice")]
pub enum Choice {
    Text(String),
    #[armonik(inlined)]
    Simple {
        name: String,
        nope: i32,
    },
    #[armonik(present)]
    Flag,
}
