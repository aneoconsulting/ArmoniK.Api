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

fn main() -> Result<(), Box<dyn Error>> {
    for proto in PROTO_FILES {
        println!("cargo:rerun-if-changed={proto}");
    }
    println!("cargo:rerun-if-changed=protos/V1");

    // Compile the descriptor set with protox (pure Rust, no protoc required).
    let fds = protox::compile(PROTO_FILES, ["protos/V1"])?;
    let bytes = fds.encode_to_vec();

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    // Input of the armonik-macros expansions, and, through the `wire` module's `DESCRIPTOR` const,
    // of the differential harness.
    write_if_changed(&out_dir.join("descriptor.bin"), &bytes)?;

    // Staleness anchor: included in the crate through `include!` so that any descriptor change
    // invalidates the crate in rustc's dep-info, and cross-checked by a const-assert emitted by
    // every derive.
    let fingerprint = {
        use std::hash::Hasher as _;
        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&bytes);
        hasher.finish()
    };
    write_if_changed(
        &out_dir.join("schema_meta.rs"),
        format!("pub(crate) const DESCRIPTOR_FINGERPRINT: u64 = {fingerprint:#018x};\n").as_bytes(),
    )?;

    Ok(())
}

fn write_if_changed(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    std::fs::write(path, contents)
}
