use std::error::Error;
use std::path::{Path, PathBuf};

use prost::Message;

/// Proto files compiled into the descriptor set.
const PROTO_FILES: &[&str] = &[
    "protos/V1/agent_common.proto",
    "protos/V1/agent_service.proto",
    "protos/V1/applications_common.proto",
    "protos/V1/applications_fields.proto",
    "protos/V1/applications_filters.proto",
    "protos/V1/applications_service.proto",
    "protos/V1/auth_common.proto",
    "protos/V1/auth_service.proto",
    "protos/V1/events_common.proto",
    "protos/V1/events_service.proto",
    "protos/V1/filters_common.proto",
    "protos/V1/objects.proto",
    "protos/V1/health_checks_common.proto",
    "protos/V1/health_checks_service.proto",
    "protos/V1/partitions_common.proto",
    "protos/V1/partitions_fields.proto",
    "protos/V1/partitions_filters.proto",
    "protos/V1/partitions_service.proto",
    "protos/V1/result_status.proto",
    "protos/V1/results_common.proto",
    "protos/V1/results_fields.proto",
    "protos/V1/results_filters.proto",
    "protos/V1/results_service.proto",
    "protos/V1/session_status.proto",
    "protos/V1/sessions_common.proto",
    "protos/V1/sessions_fields.proto",
    "protos/V1/sessions_filters.proto",
    "protos/V1/sessions_service.proto",
    "protos/V1/sort_direction.proto",
    "protos/V1/submitter_common.proto",
    "protos/V1/submitter_service.proto",
    "protos/V1/task_status.proto",
    "protos/V1/tasks_common.proto",
    "protos/V1/tasks_fields.proto",
    "protos/V1/tasks_filters.proto",
    "protos/V1/tasks_service.proto",
    "protos/V1/versions_common.proto",
    "protos/V1/versions_service.proto",
    "protos/V1/worker_common.proto",
    "protos/V1/worker_service.proto",
];

/// Proto messages implemented directly by armonik types instead of being
/// generated: each entry suppresses the generation of the message and
/// rewrites the signatures of the client/server stubs that reference it.
///
/// Flipped service by service during the direct-wire migration.
const EXTERN_TYPES: &[(&str, &str)] = &[
    (
        ".armonik.api.grpc.v1.agent.CreateTaskRequest",
        "crate::agent::create_tasks::Request",
    ),
    (
        ".armonik.api.grpc.v1.agent.CreateTaskReply",
        "crate::agent::create_tasks::Response",
    ),
    (".armonik.api.grpc.v1.Configuration", "crate::Configuration"),
    (".armonik.api.grpc.v1.Count", "crate::Count"),
    (".armonik.api.grpc.v1.DataChunk", "crate::DataChunk"),
    (".armonik.api.grpc.v1.Error", "crate::Error"),
    (".armonik.api.grpc.v1.FilterArray", "crate::FilterArray"),
    (".armonik.api.grpc.v1.FilterBoolean", "crate::FilterBoolean"),
    (".armonik.api.grpc.v1.FilterDate", "crate::FilterDate"),
    (
        ".armonik.api.grpc.v1.FilterDuration",
        "crate::FilterDuration",
    ),
    (".armonik.api.grpc.v1.FilterNumber", "crate::FilterNumber"),
    (".armonik.api.grpc.v1.FilterString", "crate::FilterString"),
    (
        ".armonik.api.grpc.v1.InitKeyedDataStream",
        "crate::InitKeyedDataStream",
    ),
    (
        ".armonik.api.grpc.v1.InitTaskRequest",
        "crate::InitTaskRequest",
    ),
    (".armonik.api.grpc.v1.Output", "crate::Output"),
    (".armonik.api.grpc.v1.ResultRequest", "crate::ResultRequest"),
    (".armonik.api.grpc.v1.Session", "crate::Session"),
    (".armonik.api.grpc.v1.StatusCount", "crate::StatusCount"),
    (".armonik.api.grpc.v1.TaskError", "crate::TaskError"),
    (".armonik.api.grpc.v1.TaskId", "crate::TaskId"),
    (".armonik.api.grpc.v1.TaskIdList", "crate::TaskIdList"),
    (
        ".armonik.api.grpc.v1.TaskIdWithStatus",
        "crate::TaskIdWithStatus",
    ),
    (".armonik.api.grpc.v1.TaskList", "crate::TaskList"),
    (".armonik.api.grpc.v1.TaskOptions", "crate::TaskOptions"),
    (
        ".armonik.api.grpc.v1.TaskOutputRequest",
        "crate::TaskOutputRequest",
    ),
    (".armonik.api.grpc.v1.TaskRequest", "crate::TaskRequest"),
    (
        ".armonik.api.grpc.v1.TaskRequestHeader",
        "crate::TaskRequestHeader",
    ),
    (
        ".armonik.api.grpc.v1.versions.ListVersionsRequest",
        "crate::versions::list::Request",
    ),
    (
        ".armonik.api.grpc.v1.versions.ListVersionsResponse",
        "crate::versions::list::Response",
    ),
    (".armonik.api.grpc.v1.auth.User", "crate::auth::User"),
    (
        ".armonik.api.grpc.v1.auth.GetCurrentUserRequest",
        "crate::auth::current_user::Request",
    ),
    (
        ".armonik.api.grpc.v1.auth.GetCurrentUserResponse",
        "crate::auth::current_user::Response",
    ),
    (
        ".armonik.api.grpc.v1.health_checks.CheckHealthRequest",
        "crate::health_checks::check::Request",
    ),
    (
        ".armonik.api.grpc.v1.health_checks.CheckHealthResponse",
        "crate::health_checks::check::Response",
    ),
    (
        ".armonik.api.grpc.v1.applications.ApplicationRaw",
        "crate::applications::Raw",
    ),
    (
        ".armonik.api.grpc.v1.applications.Filters",
        "crate::applications::filter::Or",
    ),
    (
        ".armonik.api.grpc.v1.applications.FiltersAnd",
        "crate::applications::filter::And",
    ),
    (
        ".armonik.api.grpc.v1.applications.FilterField",
        "crate::applications::filter::Field",
    ),
    (
        ".armonik.api.grpc.v1.applications.ListApplicationsRequest",
        "crate::applications::list::Request",
    ),
    (
        ".armonik.api.grpc.v1.applications.ListApplicationsRequest.Sort",
        "crate::applications::Sort",
    ),
    (
        ".armonik.api.grpc.v1.applications.ListApplicationsResponse",
        "crate::applications::list::Response",
    ),
    (
        ".armonik.api.grpc.v1.partitions.PartitionRaw",
        "crate::partitions::Raw",
    ),
    (
        ".armonik.api.grpc.v1.partitions.Filters",
        "crate::partitions::filter::Or",
    ),
    (
        ".armonik.api.grpc.v1.partitions.FiltersAnd",
        "crate::partitions::filter::And",
    ),
    (
        ".armonik.api.grpc.v1.partitions.FilterField",
        "crate::partitions::filter::Field",
    ),
    (
        ".armonik.api.grpc.v1.partitions.GetPartitionRequest",
        "crate::partitions::get::Request",
    ),
    (
        ".armonik.api.grpc.v1.partitions.GetPartitionResponse",
        "crate::partitions::get::Response",
    ),
    (
        ".armonik.api.grpc.v1.partitions.ListPartitionsRequest",
        "crate::partitions::list::Request",
    ),
    (
        ".armonik.api.grpc.v1.partitions.ListPartitionsRequest.Sort",
        "crate::partitions::Sort",
    ),
    (
        ".armonik.api.grpc.v1.partitions.ListPartitionsResponse",
        "crate::partitions::list::Response",
    ),
];

fn main() -> Result<(), Box<dyn Error>> {
    for proto in PROTO_FILES {
        println!("cargo:rerun-if-changed={proto}");
    }
    println!("cargo:rerun-if-changed=protos/V1");

    // Compile the descriptor set with protox (pure Rust, no protoc required).
    let fds = protox::compile(PROTO_FILES, ["protos/V1"])?;
    let bytes = fds.encode_to_vec();

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    // Input of the armonik-macros derives.
    write_if_changed(&out_dir.join("descriptor.bin"), &bytes)?;

    // Staleness anchor: included in the crate through `include!` so that any
    // descriptor change invalidates the crate in rustc's dep-info, and
    // cross-checked by a const-assert emitted by every derive.
    let fingerprint = fnv1a_128(&bytes);
    write_if_changed(
        &out_dir.join("schema_meta.rs"),
        format!("pub(crate) const DESCRIPTOR_FINGERPRINT: u128 = {fingerprint:#034x};\n")
            .as_bytes(),
    )?;

    // Generate the tonic stubs (and, until the migration flips them, the
    // message types) from the same descriptor set.
    let mut builder = tonic_prost_build::configure()
        .use_arc_self(true)
        .build_client(cfg!(feature = "_gen-client"))
        .build_server(cfg!(feature = "_gen-server"));
    for (proto_path, rust_path) in EXTERN_TYPES {
        builder = builder.extern_path(*proto_path, *rust_path);
    }
    builder.compile_fds(fds)?;

    Ok(())
}

fn write_if_changed(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    std::fs::write(path, contents)
}

/// FNV-1a, 128-bit.
///
/// Keep in sync with `armonik-macros/src/descriptor.rs`: a mismatch makes the
/// fingerprint const-assert emitted by every derive fail, so a divergence
/// cannot go unnoticed.
fn fnv1a_128(bytes: &[u8]) -> u128 {
    const OFFSET_BASIS: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013b;
    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash ^= byte as u128;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}
