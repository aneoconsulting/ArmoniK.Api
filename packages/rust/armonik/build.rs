use std::path::{Path, PathBuf};

/// Proto files to compile, relative to the proto root returned by [`proto_root`].
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

/// Locate the directory holding the `V1` proto package.
///
/// `protos` is a symlink to the repository-level `Protos` directory, which is what makes
/// `include = ["protos/**"]` vendor the protos into the published crate. Git only materialises that
/// symlink when `core.symlinks` is enabled, which is not the default on Windows; there it is
/// checked out as a regular file containing the link target. Both layouts have to work, so the
/// candidates are tried in order:
///
/// 1. `protos/V1` — a real directory: working symlink, or the published crate.
/// 2. The path named inside `protos` when it is a regular file — Git on Windows without symlinks.
/// 3. `../../../Protos` — the in-repository location, as a last resort.
fn proto_root(manifest_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let link = manifest_dir.join("protos");

    if link.join("V1").is_dir() {
        return Ok(link);
    }

    if link.is_file() {
        let target = std::fs::read_to_string(&link)?;
        let resolved = manifest_dir.join(target.trim());
        if resolved.join("V1").is_dir() {
            return Ok(resolved);
        }
    }

    let repository = manifest_dir.join("../../../Protos");
    if repository.join("V1").is_dir() {
        return Ok(repository);
    }

    Err(format!(
        "Could not locate the `V1` proto directory. Tried `{}` (as a directory and as a symlink \
         file) and `{}`. On Windows, enable Git symlink support with `git config core.symlinks \
         true` and re-checkout, or run from a full repository clone.",
        link.display(),
        repository.display(),
    )
    .into())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let proto_include = proto_root(&manifest_dir)?.join("V1");

    let proto_files = PROTO_FILES
        .iter()
        .map(|file| proto_include.join(file))
        .collect::<Vec<_>>();

    for file in &proto_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("protos").display()
    );
    println!("cargo:rerun-if-changed=build.rs");

    tonic_prost_build::configure()
        .use_arc_self(true)
        .build_client(cfg!(feature = "_gen-client"))
        .build_server(cfg!(feature = "_gen-server"))
        .compile_protos(&proto_files, &[proto_include])?;
    Ok(())
}
