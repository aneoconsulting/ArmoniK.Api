#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// Two modes that cannot both apply: resolving one and dropping the other frames the value a
// submessage level away from where the type says it is.
//
// `#[derive(Debug)]` and not `Default`: the salvage stub skips the `Msg` impl for a type that does
// not derive `Default`, and with it the case would carry a second, unrelated error.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(generic, transparent)]
pub struct Wrapper<T> {
    #[armonik(tag = 1)]
    pub value: T,
}
