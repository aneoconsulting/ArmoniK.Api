#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `fixture.SharedInline` has a non-oneof `token` that every variant must carry, and a member that
// is a message, so `inlined` has fields to spread. The two sets would land in one variant sharing a
// binding namespace, with tags from two different messages; rejected rather than supported, because
// no site wants the shape and making it work needs a second naming scheme for the bindings.
//
// Without the check this resolved and then emitted patterns that do not compile, with rustc
// suggesting you append `, ..` to the `#[armonik_macros::message]` attribute.
#[armonik_macros::message("fixture.SharedInline")]
#[derive(Debug)]
pub enum Pick {
    #[armonik(inlined)]
    Simple {
        token: String,
        name: String,
        count: i32,
    },
}
