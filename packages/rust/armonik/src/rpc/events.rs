super::service! {
    Events in crate::events @ "armonik.api.grpc.v1.events.Events";

    rpc GetEvents(subscribe::Request) -> stream subscribe::Response;
}
