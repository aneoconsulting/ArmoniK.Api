#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Nope";

    rpc Get(get::Request) -> get::Response;
}
