#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");



#[armonik_macros::enumeration]
#[derive(Debug, Default)]
#[armonik(enum = "fixture.Colour")]
pub struct Colour {
    pub value: i32,
}
