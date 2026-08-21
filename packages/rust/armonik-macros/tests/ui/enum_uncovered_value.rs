#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");



// `COLOUR_GREEN` (= 2) has no variant.
#[armonik_macros::enumeration("fixture.Colour")]
#[derive(Debug, Clone, Copy)]
pub enum Colour {
    Red,
    Unknown(UnknownColour),
}
