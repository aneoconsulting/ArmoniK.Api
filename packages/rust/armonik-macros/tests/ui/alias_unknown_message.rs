#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

pub struct Wrapper<T>(pub T);

// The name used to be taken on trust and only registered, so a typo here surfaced as four failing
// differential tests, the least cryptic of which reported an unmapped message and named neither
// this line nor the typo.
#[armonik_macros::alias("fixture.Simplee")]
pub type Simple = Wrapper<u32>;
