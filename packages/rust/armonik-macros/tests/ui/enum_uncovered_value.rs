include!("../support/prelude.rs");



// `COLOUR_GREEN` (= 2) has no variant.
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[armonik(enum = "fixture.Colour")]
pub enum Colour {
    Red,
    Unknown(UnknownColour),
}
