#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// An enum is oneof-shaped, so it would otherwise dispatch to the oneof-shaped resolver: the pair is
// rejected ahead of that, whichever shape would have won.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(generic, transparent)]
pub enum Both<T> {
    Text(T),
}
