#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

pub struct Wrapper<T>(pub T);

// Resolved here rather than only registered: unresolved, a typo surfaces as failing differential
// tests that name neither this line nor the typo.
#[armonik_macros::alias("fixture.Simplee")]
pub type Simple = Wrapper<u32>;
