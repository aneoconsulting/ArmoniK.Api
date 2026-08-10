include!("../support/prelude.rs");



// proto3 enums are open, so an unknown value must have somewhere to go.
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[armonik(enum = "fixture.Colour")]
pub enum Colour {
    Unspecified,
    Red,
    Green,
}
