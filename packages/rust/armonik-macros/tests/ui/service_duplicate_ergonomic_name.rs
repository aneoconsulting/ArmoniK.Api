#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// Both rpc lines would derive the method name `get` from their request module.
armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Fixture";

    rpc Get(get::Request) -> get::Response;
    rpc Watch(get::Request) -> stream get::Response;
    rpc Push(stream push::Request) -> push::Response manual;
}
