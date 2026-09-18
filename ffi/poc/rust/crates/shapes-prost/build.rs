//! Compile `ffi/schema/generated/shapes.proto` with protox (pure Rust: this container has no
//! protoc) and hand the descriptor set to prost-build.
//!
//! `btree_map` on every map field is deliberate. The canonical form in `ffi/schema/README.md`
//! sorts map entries by key; prost's default `HashMap` iterates in an unspecified order, so a
//! `HashMap` arm could not produce canonical bytes at all. `BTreeMap` is prost's supported
//! spelling for that, and it is what the validation compares against the manifest.
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let schema = manifest
        .join("../../../../schema/generated")
        .canonicalize()?;
    let proto = schema.join("shapes.proto");
    println!("cargo:rerun-if-changed={}", proto.display());

    let fds = protox::compile([&proto], [&schema])?;

    // Also written out so a test can drive a second, independent protobuf implementation
    // (prost-reflect) over the same descriptor.
    {
        use protox::prost::Message as _;
        let out = PathBuf::from(std::env::var("OUT_DIR")?);
        std::fs::write(out.join("descriptor.bin"), fds.encode_to_vec())?;
    }

    let mut cfg = prost_build::Config::new();
    cfg.btree_map(["."]);
    cfg.compile_fds(fds)?;
    Ok(())
}
