super::service! {
    Applications in crate::applications @ "armonik.api.grpc.v1.applications.Applications";

    rpc ListApplications(list::Request) -> list::Response;
}
