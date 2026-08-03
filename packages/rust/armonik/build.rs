//! Transition glue, deleted when `armonik-types` merges into this crate: copy
//! the descriptor compiled by `armonik-types`' build script (and its
//! fingerprint anchor, same construction) into this crate's `OUT_DIR`, so the
//! `service!` invocations in `src/rpc/` can expand and tripwire here too.

use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let bytes = armonik_types::wire::DESCRIPTOR;
    write_if_changed(&out_dir.join("descriptor.bin"), bytes)?;

    let fingerprint = {
        use std::hash::Hasher as _;
        let mut hasher = fnv::FnvHasher::default();
        hasher.write(bytes);
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
