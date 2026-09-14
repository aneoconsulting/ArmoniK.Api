use armonik::health_checks;
use armonik::server::HealthChecksServiceExt;

#[macro_use]
mod common;

rpc_tests! {
    client: into_health_checks;
    server: HealthChecksService, health_checks_server;
    mock: "HealthChecks";

    rpc unary check {
        request: health_checks::check::Request {},
        respond: |_request| health_checks::check::Response {
            services: vec![health_checks::ServiceHealth {
                name: String::from("rpc-check-output"),
                message: String::from("rpc-check-message"),
                health: health_checks::Status::Degraded,
            }],
        },
        convenience: check(),
        project: |response| response.services,
        check: |services| {
            assert_eq!(services[0].name, "rpc-check-output");
            assert_eq!(services[0].message, "rpc-check-message");
            assert_eq!(services[0].health, health_checks::Status::Degraded);
        },
    }
}
