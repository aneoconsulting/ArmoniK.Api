include!("../support/prelude.rs");

// `Get` is unary in the proto.
armonik_macros::service! {
    Fixture in crate::fixture @ "fixture.Fixture";

    rpc Get(stream get::Request) -> get::Response manual;
    rpc Watch(watch::Request) -> stream watch::Response;
    rpc Push(stream push::Request) -> push::Response manual;
}
