include!("../support/prelude.rs");



#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[armonik(enum = "fixture.Nope")]
pub enum Colour {
    Red,
    Unknown(UnknownColour),
}
