//! The proto-name → Rust-path map for the tonic stubs, harvested from the
//! `#[armonik(message = ...)]` annotations at compile time.
//!
//! Every `#[derive(Message)]`/`#[derive(Enum)]` registers one entry per proto
//! name into `EXTERN_MAP` (under the `_extern-map` feature, which the
//! `armonik` crate's build-dependency on this crate enables). `armonik`'s
//! build script reads `extern_mapping` to resolve its stubs' extern types
//! instead of hand-maintaining the ~150-entry list, and reads `DESCRIPTOR`
//! to prune the stub generation. See `armonik/build.rs`.

/// The full protobuf descriptor set (encoded `FileDescriptorSet`), embedded
/// from this crate's build script. `armonik`'s build script decodes it,
/// prunes it, and feeds it to `tonic-prost-build`.
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

/// `(proto full name, Rust path)` pairs, one per registered message.
///
/// The Rust path is the type's *definition* path (`module_path!`-derived,
/// e.g. `armonik_types::objects::applications::raw::Raw`), which is why the
/// object modules are `pub`. The entries are unordered and may contain
/// several messages mapping to `armonik.api.grpc.v1.Empty` (the synthetic
/// per-site empty types) — `armonik`'s build script filters those out and
/// supplies the handful of names that cannot come from annotations.
#[linkme::distributed_slice]
pub static EXTERN_MAP: [(&str, &str)];

/// The `EXTERN_MAP` entries, sorted and de-duplicated.
pub fn extern_mapping() -> Vec<(&'static str, &'static str)> {
    let mut entries: Vec<_> = EXTERN_MAP.iter().copied().collect();
    entries.sort_unstable();
    entries.dedup();
    entries
}
