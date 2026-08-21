#![allow(unexpected_cfgs)] // the expansions cfg on armonik's features; see tests/ui.rs

include!("../support/prelude.rs");

// `Push` is neither declared nor listed as unexposed.
armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Fixture";

    rpc Get(get::Request) -> get::Response;
    rpc Watch(watch::Request) -> stream watch::Response;
}
