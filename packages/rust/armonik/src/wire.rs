//! The self-registering type registry, harvested from the
//! `#[armonik(message = ...)]` / `#[armonik(absorbs = ...)]` annotations and
//! the `service!` `unexposed(...)` declarations at compile time.
//!
//! Every `#[derive(Message)]`/`#[derive(Enum)]` (and the two hand-written
//! impls) registers one [`Registration`] per proto name into [`REGISTRY`], a
//! single `linkme` distributed slice. Its consumer is the differential
//! harness, which discovers every type's round-trip and `Normalize`
//! projection through the [`Diff`] hooks (see [`crate::differential::entries`]),
//! and whose coverage ratchet walks the registered proto names against
//! [`DESCRIPTOR`].

/// The full protobuf descriptor set (encoded `FileDescriptorSet`), embedded
/// from the build script.
pub const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin"));

/// Messages present in the schema but referenced by nothing — no RPC, no
/// containing message. They have no Rust type, and the coverage ratchet
/// allows them from this list.
pub const UNREFERENCED_MESSAGES: &[&str] = &["armonik.api.grpc.v1.submitter.SessionList"];

/// How a proto name is realized on the Rust side.
pub enum Role {
    /// A Rust type implements this proto message directly.
    Message,
    /// No Rust type stands for this message — a flattening construct absorbs
    /// it into a parent. The differential harness counts it as covered
    /// through the parent.
    Absorbed,
    /// No Rust type stands for this message — it belongs to an RPC the crate
    /// deliberately does not expose (the router answers UNIMPLEMENTED for its
    /// path). Registered by `service!` from the `unexposed(...)` declaration,
    /// so this allowlist cannot drift from the RPC one.
    Unexposed,
}

/// Test-only round-trip and projection hooks for a registered type (see
/// [`crate::differential`]).
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

/// One registration: a proto name, how Rust realizes it, and its harness
/// hooks (`None` only for the type-less [`Role::Absorbed`] and
/// [`Role::Unexposed`]).
pub struct Registration {
    pub proto: &'static str,
    pub role: Role,
    pub diff: Option<Diff>,
}

#[linkme::distributed_slice]
pub static REGISTRY: [Registration];

fn collect(role: fn(&Role) -> bool) -> Vec<&'static str> {
    let mut entries: Vec<&'static str> = REGISTRY
        .iter()
        .filter_map(|r| role(&r.role).then_some(r.proto))
        .collect();
    entries.sort_unstable();
    entries.dedup();
    entries
}

/// The [`Role::Absorbed`] proto names, sorted and de-duplicated.
pub fn absorbed() -> Vec<&'static str> {
    collect(|role| matches!(role, Role::Absorbed))
}

/// The [`Role::Unexposed`] proto names, sorted and de-duplicated.
pub fn unexposed() -> Vec<&'static str> {
    collect(|role| matches!(role, Role::Unexposed))
}
