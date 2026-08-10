include!("../support/prelude.rs");



#[armonik_macros::enumeration]
#[derive(Debug, Default)]
#[armonik(enum = "fixture.Colour")]
pub struct Colour {
    pub value: i32,
}
