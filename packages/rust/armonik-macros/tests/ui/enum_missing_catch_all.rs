#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");



// proto3 enums are open, so an unknown value must have somewhere to go.
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy)]
#[armonik(enum = "fixture.Colour")]
pub enum Colour {
    Unspecified,
    Red,
    Green,
}
