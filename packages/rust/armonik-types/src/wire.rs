//! The self-registering type registry, harvested from the
//! `#[armonik(message = ...)]` / `#[armonik(replace(...))]` / `#[armonik(absorbs
//! = ...)]` annotations at compile time.
//!
//! Every `#[derive(Message)]`/`#[derive(Enum)]` (and the two hand-written
//! impls) registers one [`Registration`] per proto name into [`REGISTRY`], a
//! single `linkme` distributed slice. There are two consumers:
//!
//! - `armonik`'s build script resolves its tonic stubs' extern types and prunes
//!   its stub descriptor, through [`extern_mapping`], [`replacements`] and
//!   [`absorbed`] — enabled through the `_registry` feature the build-dependency
//!   turns on;
//! - the differential harness discovers every type's round-trip and `Normalize`
//!   projection through the `_differential`-gated [`Diff`] hooks (see
//!   [`crate::differential::entries`]).
//!
//! One slice, one entry shape. `Diff` is behind `_differential`, so the base
//! `_registry` build (the build-dependency) pulls only `linkme`, never
//! `prost-reflect`.

/// The full protobuf descriptor set (encoded `FileDescriptorSet`), embedded
/// from this crate's build script. `armonik`'s build script decodes it,
/// prunes it, and feeds it to `tonic-prost-build`.
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

/// Messages of RPCs the crate deliberately does not expose (tonic answers
/// UNIMPLEMENTED for the pruned paths). They have no Rust type and no
/// flattening parent, so both consumers must account for them from one list:
/// `armonik`'s build script prunes them from the stubs, and the differential
/// coverage ratchet allows them — so the two cannot silently drift apart.
pub const UNEXPOSED_RPC_MESSAGES: &[&str] = &[
    "armonik.api.grpc.v1.results.WatchResultRequest",
    "armonik.api.grpc.v1.results.WatchResultResponse",
    "armonik.api.grpc.v1.submitter.SessionList",
    "armonik.api.grpc.v1.submitter.WatchResultRequest",
    "armonik.api.grpc.v1.submitter.WatchResultStream",
];

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

/// How a proto name is realized on the Rust side.
pub enum Role {
    /// A Rust type at `rust_path` implements this proto message directly.
    Message {
        /// The type's definition path (`module_path!`-derived).
        rust_path: &'static str,
    },
    /// This RPC site substitutes the (shared) proto message; see [`Replacement`].
    ///
    /// A shared wire message keeps exactly one canonical [`Role::Message`]
    /// claimant; every *other* RPC that uses it must `replace(...)` (or, like
    /// `Empty`, all of its sites replace and it keeps none). Two canonical
    /// claimants for one proto name is a build error — `guard_unique_extern` in
    /// `armonik/build.rs` catches it.
    Replace(Replacement),
    /// No Rust type stands for this message — a flattening construct absorbs it
    /// into a parent. `armonik`'s build script prunes it from the stubs and the
    /// differential harness counts it as covered through the parent.
    Absorbed,
}

/// Test-only round-trip and projection hooks for a registered type (see
/// [`crate::differential`]). Behind `_differential` so the base build never
/// names `prost_reflect`.
#[cfg(feature = "_differential")]
#[derive(Clone, Copy)]
pub struct Diff {
    /// Decode the bytes as the armonik type and re-encode them.
    pub roundtrip: fn(&[u8]) -> Result<Vec<u8>, ::prost::DecodeError>,
    /// Canonical encoding of the type's `Default` (the zero-default invariant,
    /// and the harness's canonical-absence fold).
    pub default_encoding: fn() -> Vec<u8>,
    /// The type's value-level projection, from its [`crate::differential::Normalize`] impl.
    pub normalize: fn(&mut crate::differential::prost_reflect::DynamicMessage),
}

/// One registration: a proto name, how Rust realizes it, and — under
/// `_differential` — its harness hooks (`None` only for [`Role::Absorbed`],
/// which has no Rust type).
pub struct Registration {
    pub proto: &'static str,
    pub role: Role,
    #[cfg(feature = "_differential")]
    pub diff: Option<Diff>,
}

#[linkme::distributed_slice]
pub static REGISTRY: [Registration];

/// `(proto full name, Rust definition path)` pairs, one per registered message,
/// sorted and de-duplicated.
///
/// The Rust path is the type's *definition* path (`module_path!`-derived, e.g.
/// `armonik_types::objects::applications::raw::Raw`), which is why the object
/// modules are `pub`. Types carrying `#[armonik(replace(...))]` do not appear
/// here — they register a [`Replacement`] instead — so the shared proto
/// messages they stand for (e.g. `Empty`) stay unambiguously keyed.
pub fn extern_mapping() -> Vec<(&'static str, &'static str)> {
    let mut entries: Vec<(&'static str, &'static str)> = REGISTRY
        .iter()
        .filter_map(|r| match &r.role {
            Role::Message { rust_path } => Some((r.proto, *rust_path)),
            _ => None,
        })
        .collect();
    entries.sort_unstable();
    entries.dedup();
    entries
}

/// The [`Replacement`] entries, sorted and de-duplicated.
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
    let mut entries: Vec<&'static Replacement> = REGISTRY
        .iter()
        .filter_map(|r| match &r.role {
            Role::Replace(spec) => Some(spec),
            _ => None,
        })
        .collect();
    entries.sort_unstable_by_key(|r| key(r));
    entries.dedup_by_key(|r| key(r));
    entries
}

/// The [`Role::Absorbed`] proto names, sorted and de-duplicated.
pub fn absorbed() -> Vec<&'static str> {
    let mut entries: Vec<&'static str> = REGISTRY
        .iter()
        .filter_map(|r| match &r.role {
            Role::Absorbed => Some(r.proto),
            _ => None,
        })
        .collect();
    entries.sort_unstable();
    entries.dedup();
    entries
}
