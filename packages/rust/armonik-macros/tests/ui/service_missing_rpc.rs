include!("../support/prelude.rs");

// `Push` is neither declared nor listed as unexposed.
armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Fixture";

    rpc Get(get::Request) -> get::Response;
    rpc Watch(watch::Request) -> stream watch::Response;
}
