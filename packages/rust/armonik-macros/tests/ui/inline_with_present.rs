include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Choice", oneof = "choice")]
pub enum Choice {
    Text(String),
    Simple(String),
    #[armonik(present, inline)]
    Flag,
}
