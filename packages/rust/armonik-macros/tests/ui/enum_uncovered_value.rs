#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");



// `COLOUR_GREEN` (= 2) has no variant.
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy)]
#[armonik(enum = "fixture.Colour")]
pub enum Colour {
    Red,
    Unknown(UnknownColour),
}
