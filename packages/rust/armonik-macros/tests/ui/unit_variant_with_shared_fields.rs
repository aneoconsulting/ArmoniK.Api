#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A bare unit variant in an enum whose message has non-oneof fields: it means "no member set", but
// every variant of a whole-message enum has to carry the shared fields, and this one carries none.
// The complaint is about the fields it dropped, not about a member name it never meant to give.
#[armonik_macros::message("fixture.Shared")]
#[derive(Debug)]
pub enum Shared {
    Text { token: String, text: String },
    Flag { token: String, flag: bool },
    None,
}
