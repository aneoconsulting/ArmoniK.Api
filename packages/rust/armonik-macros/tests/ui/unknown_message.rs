include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(message = "fixture.Nope")]
pub struct Simple {
    pub name: String,
}
