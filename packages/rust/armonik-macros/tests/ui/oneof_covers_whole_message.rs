// The stub a failed expansion emits is gated on `_differential`, which `armonik` declares
// and a one-file compile-fail crate cannot.
#![allow(unexpected_cfgs)]

include!("../support/prelude.rs");

// `pick` is all of `fixture.OnlyOneof`, so naming it adds nothing.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.OnlyOneof", oneof = "pick")]
pub enum OnlyOneof {
    First(String),
    Second(String),
}
