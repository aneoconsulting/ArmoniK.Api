include!("../support/prelude.rs");

// A descriptor-validated field takes its tag from the descriptor; `tag` belongs to `generic` mode.
#[armonik_macros::message]
#[derive(Debug, Default)]
#[armonik(message = "fixture.Simple")]
pub struct Simple {
    #[armonik(tag = 1)]
    pub name: String,
    pub count: i32,
}
