#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

#[armonik_macros::message("fixture.TwoOneofs")]
#[derive(Debug)]
pub enum TwoOneofs {
    A(String),
    B(String),
}
