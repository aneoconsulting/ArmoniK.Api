#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// The expansion implements these five in terms of the proto value, so that a named variant and the
// catch-all holding its number are one value. Deriving one is rejected here rather than left to
// rustc's E0119 at the attribute, which says two impls collide without saying which to keep.
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[armonik(enum = "fixture.Colour")]
pub enum Colour {
    Red,
    Green,
    Unknown(UnknownColour),
}
