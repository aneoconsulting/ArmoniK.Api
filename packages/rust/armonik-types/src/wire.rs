//! The self-registering type registry, harvested from the
//! `#[armonik(message = ...)]` / `#[armonik(absorbs = ...)]` annotations at
//! compile time.
//!
//! Every `#[derive(Message)]`/`#[derive(Enum)]` (and the two hand-written
//! impls) registers one [`Registration`] per proto name into [`REGISTRY`], a
//! single `linkme` distributed slice. Its consumer is the differential
//! harness, which discovers every type's round-trip and `Normalize`
//! projection through the `_differential`-gated [`Diff`] hooks (see
//! [`crate::differential::entries`]), and whose coverage ratchet walks the
//! registered proto names against [`DESCRIPTOR`].
//!
//! One slice, one entry shape. `Diff` is behind `_differential`, so a base
//! `_registry` build pulls only `linkme`, never `prost-reflect`.

/// The full protobuf descriptor set (encoded `FileDescriptorSet`), embedded
/// from this crate's build script.
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

/// Messages of RPCs the crate deliberately does not expose (the router
/// answers UNIMPLEMENTED for their paths). They have no Rust type and no
/// flattening parent; the differential coverage ratchet allows them from this
/// list. Retired at the crate merge: `service!` emits it from the
/// `unexposed(...)` declarations.
pub const UNEXPOSED_RPC_MESSAGES: &[&str] = &[
    "armonik.api.grpc.v1.results.WatchResultRequest",
    "armonik.api.grpc.v1.results.WatchResultResponse",
    "armonik.api.grpc.v1.submitter.SessionList",
    "armonik.api.grpc.v1.submitter.WatchResultRequest",
    "armonik.api.grpc.v1.submitter.WatchResultStream",
];

/// How a proto name is realized on the Rust side.
pub enum Role {
    /// A Rust type at `rust_path` implements this proto message directly.
    Message {
        /// The type's definition path (`module_path!`-derived).
        rust_path: &'static str,
    },
    /// No Rust type stands for this message — a flattening construct absorbs it
    /// into a parent. The differential harness counts it as covered through the
    /// parent.
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
