super::service! {
    HealthChecks in crate::health_checks @ "armonik.api.grpc.v1.health_checks.HealthChecksService";

    rpc CheckHealth(check::Request) -> check::Response;
}
