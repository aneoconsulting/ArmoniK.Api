#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A descriptor-validated field takes its tag from the descriptor; `tag` belongs to `generic` mode.
#[armonik_macros::message("fixture.Simple")]
#[derive(Debug, Default)]
pub struct Simple {
    #[armonik(tag = 1)]
    pub name: String,
    pub count: i32,
}
