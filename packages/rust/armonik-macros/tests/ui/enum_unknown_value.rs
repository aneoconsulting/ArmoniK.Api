#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");



// `Blue` matches no value of `fixture.Colour`.
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy)]
#[armonik(enum = "fixture.Colour")]
pub enum Colour {
    Red,
    Green,
    Blue,
    Unknown(UnknownColour),
}
