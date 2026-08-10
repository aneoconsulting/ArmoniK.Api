include!("../support/prelude.rs");

armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Nope";

    rpc Get(get::Request) -> get::Response;
}
