#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// The argument is the oneof's path, so the message name alone names no oneof. Reported as what it
// is, rather than as "no message named `fixture`" about everything before the last dot.
#[armonik_macros::oneof("fixture.Choice")]
#[derive(Debug)]
pub enum Choice {
    Text(String),
    Simple(String),
    Flag,
}
