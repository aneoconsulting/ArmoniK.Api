#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `Chat` is bidirectional in the proto, and this line claims only the request half. The two
// `stream` keywords are read independently, which is what lets a bidi line be declared at all and
// what makes half of one a mismatch on the side that is missing rather than "not bidirectional".
armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Fixture";

    rpc Get(get::Request) -> get::Response;
    rpc Watch(watch::Request) -> stream watch::Response;
    rpc Push(stream push::Request) -> push::Response;
    rpc Chat(stream chat::Request) -> chat::Response;
}
