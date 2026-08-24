#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `pick` is all of `fixture.OnlyOneof`, so naming it adds nothing.
#[armonik_macros::oneof("fixture.OnlyOneof.pick")]
#[derive(Debug)]
pub enum OnlyOneof {
    First(String),
    Second(String),
}
