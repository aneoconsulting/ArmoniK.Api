#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `present` wants a unit variant; `fixture.Shared.token` wants every variant to carry it.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Shared")]
pub enum Shared {
    Text { token: String, text: String },
    #[armonik(present)]
    Flag,
}
