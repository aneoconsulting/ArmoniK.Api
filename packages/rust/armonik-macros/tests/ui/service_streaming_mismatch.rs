#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `Get` is unary in the proto.
armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Fixture";

    rpc Get(stream get::Request) -> get::Response;
    rpc Watch(watch::Request) -> stream watch::Response;
    rpc Push(stream push::Request) -> push::Response;
}
