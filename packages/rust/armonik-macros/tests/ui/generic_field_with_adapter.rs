#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A generic type is checked only by the field-shape comparison at its alias, and an adapter has no
// shape to compare, so `with` has nowhere to be checked here.
#[armonik_macros::message]
#[derive(Debug)]
pub struct Generic<T> {
    #[armonik(tag = 1, with = "crate::codec::adapters::Own")]
    pub first: T,
}
