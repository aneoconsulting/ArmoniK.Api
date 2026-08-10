include!("../support/prelude.rs");

#[armonik_macros::message(fixture.Simple)]
#[derive(Debug, Default)]
pub struct Simple {
    pub name: String,
    pub count: i32,
}
