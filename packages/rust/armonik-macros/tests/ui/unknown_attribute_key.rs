include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(mesage = "fixture.Simple")]
pub struct Simple {
    pub name: String,
}
