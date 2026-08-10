include!("../support/prelude.rs");

// A struct variant carries the message's non-oneof fields plus *one* member payload.
#[armonik_macros::message]
#[derive(Debug)]
#[armonik(message = "fixture.Choice")]
pub enum Choice {
    Text {
        shared: String,
        text: String,
        extra: String,
    },
}
