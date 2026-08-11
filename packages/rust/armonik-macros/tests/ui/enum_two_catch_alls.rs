#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");



#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[armonik(enum = "fixture.Colour")]
pub enum Colour {
    Red,
    Green,
    Unknown(UnknownColour),
    AlsoUnknown(AlsoUnknownColour),
}
