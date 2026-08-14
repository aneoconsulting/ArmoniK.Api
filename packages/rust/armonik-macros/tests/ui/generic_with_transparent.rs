#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// Two modes that cannot both apply: `generic` used to win and `transparent` was dropped in silence,
// which frames the value one submessage level deeper than the type says it does.
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
