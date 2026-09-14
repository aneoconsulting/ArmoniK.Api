use std::error::Error;
use std::path::{Path, PathBuf};

use prost::Message;

/// Where the schema lives. A symlink to the repository-wide `Protos`, shared with the other language
/// bindings.
const PROTO_DIR: &str = "protos/V1";

/// Every `.proto` under [`PROTO_DIR`], sorted.
///
/// Read from the directory rather than listed, because the schema is shared: a file can arrive with
/// another binding's change, and one that nothing already compiled imports would otherwise be absent
/// from the descriptor set, which is the denominator of the harness's coverage ratchet.
///
/// Sorted so the descriptor bytes, and with them `DESCRIPTOR_FINGERPRINT`, do not depend on the order
/// the filesystem hands entries back.
fn proto_files() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(PROTO_DIR)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "proto"))
        .collect();
    if files.is_empty() {
        return Err(format!("no .proto files under {PROTO_DIR}").into());
    }
    files.sort();
    Ok(files)
}

fn main() -> Result<(), Box<dyn Error>> {
    let protos = proto_files()?;
    for proto in &protos {
        println!("cargo:rerun-if-changed={}", proto.display());
    }
    // The directory itself as well, so that adding or removing a file re-runs this.
    println!("cargo:rerun-if-changed={PROTO_DIR}");

    // Compile the descriptor set with protox (pure Rust, no protoc required).
    let fds = protox::compile(&protos, [PROTO_DIR])?;
    let bytes = fds.encode_to_vec();

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    // Input of the armonik-macros expansions, and of the differential harness, which embeds it
    // with its own `include_bytes!`.
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
