// The stub a failed expansion emits is gated on `_differential`, which `armonik` declares
// and a one-file compile-fail crate cannot.
#![allow(unexpected_cfgs)]

include!("../support/prelude.rs");

#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Choice", oneof = "choise")]
pub enum Choice {
    Text(String),
}
