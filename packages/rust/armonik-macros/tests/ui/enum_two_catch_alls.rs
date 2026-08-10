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
