super::service! {
    Worker in crate::worker @ "armonik.api.grpc.v1.worker.Worker";

    rpc Process(process::Request) -> process::Response;
    rpc HealthCheck(health_check::Request) -> health_check::Response;
}
