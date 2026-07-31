//! The proto-name → Rust-path map for the tonic stubs, harvested from the
//! `#[armonik(message = ...)]` and `#[armonik(replace(...))]` annotations at
//! compile time.
//!
//! Every `#[derive(Message)]`/`#[derive(Enum)]` registers one entry per proto
//! name into `EXTERN_MAP` (under the `_extern-map` feature, which the
//! `armonik` crate's build-dependency on this crate enables). A type carrying
//! `#[armonik(replace(...))]` instead registers a [`Replacement`] into
//! `REPLACE_MAP`: it does not claim the shared proto message in `EXTERN_MAP`
//! (that stays unambiguous), and it tells the build script to give that RPC a
//! distinct synthetic message. `armonik`'s build script reads `extern_mapping`
//! and `replacements` to resolve its stubs' extern types instead of
//! hand-maintaining the list, and reads `DESCRIPTOR` to prune the stub
//! generation. See `armonik/build.rs`.

/// The full protobuf descriptor set (encoded `FileDescriptorSet`), embedded
/// from this crate's build script. `armonik`'s build script decodes it,
/// prunes it, and feeds it to `tonic-prost-build`.
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

/// `(proto full name, Rust path)` pairs, one per registered message.
///
/// The Rust path is the type's *definition* path (`module_path!`-derived,
/// e.g. `armonik_types::objects::applications::raw::Raw`), which is why the
/// object modules are `pub`. Types carrying `#[armonik(replace(...))]` do not
/// register here — they register a [`Replacement`] instead — so the shared
/// proto messages they stand for (e.g. `Empty`) stay unambiguously keyed.
#[linkme::distributed_slice]
pub static EXTERN_MAP: [(&str, &str)];

/// The `EXTERN_MAP` entries, sorted and de-duplicated.
pub fn extern_mapping() -> Vec<(&'static str, &'static str)> {
    let mut entries: Vec<_> = EXTERN_MAP.iter().copied().collect();
    entries.sort_unstable();
    entries.dedup();
    entries
}

/// Direction of an RPC message slot, as seen from the service definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The request message (`rpc M(Input) returns (...)`).
    Input,
    /// The response message (`rpc M(...) returns (Output)`).
    Output,
}

/// A per-RPC message substitution harvested from `#[armonik(replace(...))]`.
///
/// The annotated Rust type stands in for a shared proto message at one RPC
/// site. `armonik`'s build script drift-checks `message` against the live
/// descriptor, then rewrites the RPC's `direction` slot to the synthetic
/// `target` message and extern-maps `target` to `rust_path`, so RPCs sharing
/// `message` end up with distinct stub signatures pointing at distinct types.
pub struct Replacement {
    /// Proto service name (e.g. `"Tasks"`).
    pub service: &'static str,
    /// Proto method name (e.g. `"ListTasksDetailed"`).
    pub method: &'static str,
    /// Which slot of the RPC this type occupies.
    pub direction: Direction,
    /// The real proto message the slot is expected to hold today
    /// (drift-checked against the descriptor).
    pub message: &'static str,
    /// The synthetic proto message name injected into the stub descriptor and
    /// extern-mapped to `rust_path`. Absent from the real schema.
    pub target: &'static str,
    /// Definition path of the Rust type owning the slot (`module_path!`-derived).
    pub rust_path: &'static str,
}

#[linkme::distributed_slice]
pub static REPLACE_MAP: [Replacement];

/// Proto messages that a flattening construct swallows into its parent (a
/// `with` adapter's `absorbs`, a transparent enum chain's middle wrappers, an
/// inline struct variant's message), so they have no Rust type of their own.
/// `armonik`'s build script prunes them from the stub generation, and the
/// differential harness counts them as covered through their parent.
#[linkme::distributed_slice]
pub static ABSORBED: [&str];

/// The `ABSORBED` proto names, sorted and de-duplicated.
pub fn absorbed() -> Vec<&'static str> {
    let mut entries: Vec<&'static str> = ABSORBED.iter().copied().collect();
    entries.sort_unstable();
    entries.dedup();
    entries
}

/// The `REPLACE_MAP` entries, sorted and de-duplicated.
pub fn replacements() -> Vec<&'static Replacement> {
    fn key(r: &Replacement) -> (&str, &str, u8, &str, &str, &str) {
        (
            r.service,
            r.method,
            r.direction as u8,
            r.message,
            r.target,
            r.rust_path,
        )
    }
    let mut entries: Vec<&'static Replacement> = REPLACE_MAP.iter().collect();
    entries.sort_unstable_by_key(|r| key(r));
    entries.dedup_by_key(|r| key(r));
    entries
}
