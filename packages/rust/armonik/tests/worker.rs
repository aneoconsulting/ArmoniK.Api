use armonik::server::WorkerServiceExt;
use armonik::worker;

#[macro_use]
mod common;

/// Spelled out field by field, rather than filled in with `..Default`, so that a
/// field added to `ProcessRequest` has to be acknowledged here. `process` is the
/// one RPC whose convenience method takes the request message itself, so both
/// halves of the pair share this.
fn process_request() -> worker::process::Request {
    worker::process::Request {
        communication_token: String::from("rpc-process-input"),
        session_id: Default::default(),
        task_id: Default::default(),
        task_options: Default::default(),
        expected_output_keys: Default::default(),
        payload_id: Default::default(),
        data_dependencies: Default::default(),
        data_folder: Default::default(),
        configuration: Default::default(),
    }
}

rpc_tests! {
    client: into_worker;
    server: WorkerService, worker_server;
    // `ArmoniK.Api.Mock` does not implement Worker: a worker is what the cluster
    // calls, rather than part of it.
    mock: none;

    rpc unary health_check {
        request: worker::health_check::Request {},
        respond: |_request| worker::health_check::Response::Serving,
        convenience: health_check(),
        check: |response| {
            assert_eq!(response, worker::health_check::Response::Serving);
        },
    }

    rpc unary process {
        request: worker::process::Request {
            ..process_request()
        },
        respond: |_request| worker::process::Response {
            output: armonik::Output::Error {
                details: String::from("rpc-process-output"),
            },
        },
        convenience: process(process_request()),
        project: |response| response.output,
        check: |output| match output {
            armonik::Output::Ok => panic!("Unexpected ok"),
            armonik::Output::Error { details } => {
                assert_eq!(details, "rpc-process-output")
            }
        },
    }
}
