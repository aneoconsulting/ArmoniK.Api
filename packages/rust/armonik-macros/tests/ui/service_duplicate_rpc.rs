include!("../support/prelude.rs");

armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Fixture";

    rpc Get(get::Request) -> get::Response;
    rpc Get(again::Request) -> again::Response;
    rpc Watch(watch::Request) -> stream watch::Response;
    rpc Push(stream push::Request) -> push::Response manual;
}
