include!("../support/prelude.rs");

// A unified type stands for several identical protos; the struct side of that never had a user.
#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(message = "fixture.Simple", message = "fixture.Typed")]
pub struct Simple {
    pub name: String,
    pub count: i32,
}
