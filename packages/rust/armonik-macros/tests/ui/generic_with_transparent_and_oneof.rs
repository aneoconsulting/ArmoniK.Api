#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `oneof = ...` does not get to decide this one: the pair is rejected whichever shape would
// otherwise have won the dispatch.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(oneof = "pick", generic, transparent)]
pub enum Both<T> {
    Text(T),
}
