#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `transparent` names the message the type is wire-identical to, and a type with parameters stands
// for no one message: the two readings of the item cannot both hold.
//
// `#[derive(Debug)]` and not `Default`: the salvage stub skips the `Msg` impl for a type that does
// not derive `Default`, and with it the case would carry a second, unrelated error.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(transparent)]
pub struct Wrapper<T> {
    pub value: T,
}
