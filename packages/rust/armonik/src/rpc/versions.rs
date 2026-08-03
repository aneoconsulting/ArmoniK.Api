super::service! {
    Versions in crate::versions @ "armonik.api.grpc.v1.versions.Versions";

    rpc ListVersions(list::Request) -> list::Response;
}
