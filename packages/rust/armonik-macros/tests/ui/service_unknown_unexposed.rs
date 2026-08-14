#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Fixture";
    unexposed(Nope);

    rpc Get(get::Request) -> get::Response;
    rpc Watch(watch::Request) -> stream watch::Response;
    rpc Push(stream push::Request) -> push::Response;
}
