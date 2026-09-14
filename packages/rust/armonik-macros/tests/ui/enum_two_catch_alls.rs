#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");



#[armonik_macros::enumeration("fixture.Colour")]
#[derive(Debug, Clone, Copy)]
pub enum Colour {
    Red,
    Green,
    Unknown(UnknownColour),
    AlsoUnknown(AlsoUnknownColour),
}
