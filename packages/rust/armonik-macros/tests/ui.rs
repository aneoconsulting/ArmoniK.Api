//! Compile-fail suite: one case per diagnostic the derives can produce.
//!
//! The whole value of these macros is that a mistake in an annotation is a compile error *at the
//! annotation* rather than a wrong byte on the wire, and none of those errors used to be under
//! test: 92 `syn::Error::new` sites and 77 error pushes, guarded by unit tests that call the
//! resolvers directly and never look at what rustc prints.
//!
//! Each case is a minimal `tests/ui/*.rs` paired with the exact `*.stderr` rustc prints for it.
//! Regenerate after a deliberate wording change with:
//!
//! ```sh
//! TRYBUILD=overwrite cargo test -p armonik-macros --test ui
//! ```
//!
//! and read the diff: an unexpected line there is the review.
//!
//! # What the cases compile against
//!
//! `tests/fixture.proto`, compiled by this file into the target directory, rather than the ArmoniK
//! schema: a diagnostic reads better against a two-field message than against `TaskDetailed`. And
//! `tests/support/prelude.rs`, a stand-in for the crate-root surface the
//! expansions name (`crate::codec`, `crate::__schema`, `crate::register!`), because a `trybuild`
//! case is its own crate while the derives only ever expand inside `armonik`.
//!
//! # What is not here
//!
//! The const-assert classes: a field whose Rust type has the wrong shape, a stale descriptor
//! fingerprint, an rpc line naming the wrong request or response type. Those messages live in
//! `armonik::codec` and fire at const-eval against the real trait impls, so pinning them here would
//! mean copying the codec into the prelude and testing the copy. They are a different mechanism
//! from the expansion-time diagnostics this suite exists to pin.
//!
//! # Why every case allows `unexpected_cfgs`
//!
//! The expansions `cfg` on `armonik`'s own features (`serde` on the re-emitted item,
//! `_differential`-era gates on the stubs). A one-file compile-fail crate declares neither, so
//! rustc reports each as an unexpected value: ten lines of noise per case, about what a diagnostic
//! costs, in the files whose readability is the point.
//!
//! # Toolchain sensitivity
//!
//! `.stderr` snapshots generally are toolchain-sensitive, but every line in these ones is a message
//! this crate wrote, laid out by rustc's diagnostic frame; none is a rustc-owned diagnostic. Checked
//! against stable, nightly and the 1.88 MSRV, which agree exactly. If a future rustc does change the
//! frame, the fix is the `TRYBUILD=overwrite` line above.

/// Compile the fixture schema and hand the compile-fail runs an `OUT_DIR` holding it.
///
/// `descriptor::index` reads `$OUT_DIR/descriptor.bin` from the environment of the compiler that is
/// expanding the macro. `trybuild` shells out to cargo, which passes its own environment down to
/// rustc, and the generated project has no build script of its own to set the variable, so the one
/// exported here is the one the expansions see.
///
/// The prelude's `crate::__schema` reads `fixture_fingerprint.rs` out of the same directory through
/// `env!("OUT_DIR")`, which resolves to this one for the same reason.
fn compile_the_fixture_descriptor() {
    use prost::Message as _;

    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("ui-fixture");
    std::fs::create_dir_all(&dir).expect("create the fixture directory");

    let descriptor = protox::compile(["tests/fixture.proto"], ["tests"])
        .expect("compile tests/fixture.proto")
        .encode_to_vec();

    let fingerprint = {
        use std::hash::Hasher as _;
        let mut hasher = fnv::FnvHasher::default();
        hasher.write(&descriptor);
        hasher.finish()
    };

    std::fs::write(dir.join("descriptor.bin"), &descriptor).expect("write the descriptor set");
    std::fs::write(
        dir.join("fixture_fingerprint.rs"),
        format!("pub(crate) const DESCRIPTOR_FINGERPRINT: u64 = {fingerprint:#018x};\n"),
    )
    .expect("write the fingerprint");

    std::env::set_var("OUT_DIR", dir);
}

#[test]
fn ui() {
    compile_the_fixture_descriptor();
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
