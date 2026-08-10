// The stub a failed expansion emits is gated on `_differential`, which `armonik` declares
// and a one-file compile-fail crate cannot.
#![allow(unexpected_cfgs)]

include!("../support/prelude.rs");

// `present` records that a member was set; it needs a bool or an empty message, not a string.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Choice", oneof = "choice")]
pub enum Choice {
    #[armonik(present)]
    Text,
    Simple(String),
    #[armonik(present)]
    Flag,
}
