#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A type with parameters stands for no one message, so naming one is the mistake: dropping the name
// is what makes it generic, and its fields then spell their own tags.
#[armonik_macros::message("fixture.Simple")]
#[derive(Debug, Default)]
pub struct Simple<T> {
    pub name: String,
    pub count: T,
}
