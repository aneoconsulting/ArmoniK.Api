#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A generic type names no proto message, so every field must carry its own tag.
#[armonik_macros::message]
#[derive(Debug)]
pub struct Generic<T> {
    #[armonik(tag = 1)]
    pub first: T,
    pub second: String,
}
