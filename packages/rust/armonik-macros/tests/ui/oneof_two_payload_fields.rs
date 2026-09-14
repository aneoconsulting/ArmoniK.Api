#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// A struct variant carries the message's non-oneof fields plus *one* member payload.
#[armonik_macros::message("fixture.Choice")]
#[derive(Debug)]
pub enum Choice {
    Text {
        shared: String,
        text: String,
        extra: String,
    },
}
