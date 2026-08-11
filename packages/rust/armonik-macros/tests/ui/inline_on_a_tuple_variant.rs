include!("../support/prelude.rs");

// There is nothing to spread the member's fields into.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Choice", oneof = "choice")]
pub enum Choice {
    Text(String),
    #[armonik(inline)]
    Simple(String),
    #[armonik(present)]
    Flag,
}
