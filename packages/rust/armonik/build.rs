/// The proto files to compile, relative to [`PROTO_ROOT`].
///
/// A list rather than a glob, so that each file can be named to `cargo:rerun-if-changed` below.
const PROTO_FILES: &[&str] = &[
    "agent_common.proto",
    "agent_service.proto",
    "applications_common.proto",
    "applications_fields.proto",
    "applications_filters.proto",
    "applications_service.proto",
    "auth_common.proto",
    "auth_service.proto",
    "events_common.proto",
    "events_service.proto",
    "filters_common.proto",
    "objects.proto",
    "health_checks_common.proto",
    "health_checks_service.proto",
    "partitions_common.proto",
    "partitions_fields.proto",
    "partitions_filters.proto",
    "partitions_service.proto",
    "result_status.proto",
    "results_common.proto",
    "results_fields.proto",
    "results_filters.proto",
    "results_service.proto",
    "session_status.proto",
    "sessions_common.proto",
    "sessions_fields.proto",
    "sessions_filters.proto",
    "sessions_service.proto",
    "sort_direction.proto",
    "submitter_common.proto",
    "submitter_service.proto",
    "task_status.proto",
    "tasks_common.proto",
    "tasks_fields.proto",
    "tasks_filters.proto",
    "tasks_service.proto",
    "versions_common.proto",
    "versions_service.proto",
    "worker_common.proto",
    "worker_service.proto",
];

/// Where the protos live, as seen from this crate: under a symlink to the repository's `Protos`, which is
/// what makes `include = ["protos/**"]` vendor them into the published crate.
const PROTO_ROOT: &str = "protos/V1";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = PROTO_FILES
        .iter()
        .map(|file| format!("{PROTO_ROOT}/{file}"))
        .collect::<Vec<_>>();

    // With nothing declared, cargo watches the whole crate directory, so editing a test re-runs `protoc`
    // over all forty protos. Declaring them replaces that fallback rather than adding to it, which is why
    // this list now decides rebuilds as well as what gets compiled.
    for file in &proto_files {
        println!("cargo:rerun-if-changed={file}");
    }
    // The symlink itself, so that repointing it counts as a change too.
    println!("cargo:rerun-if-changed=protos");
    println!("cargo:rerun-if-changed=build.rs");

    tonic_prost_build::configure()
        .use_arc_self(true)
        .build_client(cfg!(feature = "_gen-client"))
        .build_server(cfg!(feature = "_gen-server"))
        // Both slices have to hold the same type, and `proto_files` is `Vec<String>`.
        .compile_protos(&proto_files, &[String::from(PROTO_ROOT)])?;
    Ok(())
}
