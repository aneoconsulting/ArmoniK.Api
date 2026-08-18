#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.Straddled.token` (tag 2) sits between the members at tags 1 and 3.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Straddled")]
pub enum Straddled {
    Text { token: String, text: String },
    Other { token: String, other: String },
}
