#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

#[armonik_macros::oneof("fixture.Choice.choise")]
#[derive(Debug)]
pub enum Choice {
    Text(String),
}
