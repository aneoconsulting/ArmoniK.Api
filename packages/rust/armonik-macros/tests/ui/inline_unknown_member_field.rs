include!("../support/prelude.rs");

// `fixture.Simple` has no `nope`.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Choice", oneof = "choice")]
pub enum Choice {
    Text(String),
    #[armonik(inline)]
    Simple {
        name: String,
        nope: i32,
    },
    #[armonik(present)]
    Flag,
}
