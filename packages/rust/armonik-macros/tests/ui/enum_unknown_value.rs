include!("../support/prelude.rs");



// `Blue` matches no value of `fixture.Colour`.
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[armonik(enum = "fixture.Colour")]
pub enum Colour {
    Red,
    Green,
    Blue,
    Unknown(UnknownColour),
}
