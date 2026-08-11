#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.TwoOneofs")]
pub enum TwoOneofs {
    A(String),
    B(String),
}
