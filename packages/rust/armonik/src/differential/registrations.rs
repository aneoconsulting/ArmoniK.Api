//! The self-registering type registry, harvested from the `#[armonik(message = ...)]` /
//! `#[armonik(absorbs = ...)]` annotations and the `service!` `unexposed(...)` declarations at
//! compile time.
//!
//! Every `#[armonik_macros::message]`/`#[armonik_macros::enumeration]` expansion (and the two
//! hand-written impls) registers one [`Registration`] per proto name into [`REGISTRY`], a single
//! `linkme` distributed slice. Its consumer is the differential harness next door, which discovers
//! every type's round-trip and `Normalize` projection through the [`Hooks`] a [`Role::Message`]
//! carries, and whose coverage ratchet walks the registered names against the descriptor pool.

use prost_reflect::DynamicMessage;

/// Messages present in the schema but referenced by nothing: no RPC, no containing message. They
/// have no Rust type, and the coverage ratchet allows them from this list.
pub(crate) const UNREFERENCED_MESSAGES: &[&str] = &["armonik.api.grpc.v1.submitter.SessionList"];

/// One registration: a proto name and what stands for it on the Rust side.
pub(crate) struct Registration {
    pub proto: &'static str,
    pub role: Role,
}

/// How a proto name is realized on the Rust side.
pub(crate) enum Role {
    /// A Rust type implements this proto message directly; these are the hooks the harness drives
    /// it through.
    Message(Hooks),
    /// No Rust type stands for this message: a flattening construct absorbs it into a parent,
    /// through which the harness counts it as covered.
    Absorbed,
    /// No Rust type stands for this message: it belongs to an RPC the crate deliberately does not
    /// expose (the router answers UNIMPLEMENTED for its path). Registered by `service!` from the
    /// `unexposed(...)` declaration, so this allowlist cannot drift from the RPC one.
    Unexposed,
}

/// A registered type's round-trip and projection hooks.
#[derive(Clone, Copy)]
pub(crate) struct Hooks {
    /// Decode the bytes as the armonik type and re-encode them.
    pub roundtrip: fn(&[u8]) -> Result<Vec<u8>, prost::DecodeError>,
    /// Canonical encoding of the type's `Default` (the zero-default invariant, and the harness's
    /// canonical-absence fold).
    pub default_encoding: fn() -> Vec<u8>,
    /// The type's value-level projection, from its [`super::Normalize`] impl.
    pub normalize: fn(&mut DynamicMessage),
}

#[linkme::distributed_slice]
pub(crate) static REGISTRY: [Registration];

/// One RPC of one service, recorded from one of the two ends that has to agree about it.
pub(crate) struct Rpc {
    pub service: &'static str,
    pub method: &'static str,
}

/// Every RPC a `service!` invocation declares. `unexposed(...)` ones are not here: they are declared
/// precisely because the crate does not expose them.
#[linkme::distributed_slice]
pub(crate) static DECLARED_RPCS: [Rpc];

/// Every client method that claims an RPC, recorded by `#[armonik_macros::client]`.
///
/// Two slices rather than one because the point is that they are filled from opposite ends: the
/// declaration in `rpc/*.rs` and the implementation in `client/*.rs`. The test next door asserts
/// they pair up, which is what replaces the guarantee the convenience generator used to give for
/// free.
#[linkme::distributed_slice]
pub(crate) static CLIENT_METHODS: [Rpc];

/// Every registered proto name that a Rust type implements, with that type's hooks.
pub(crate) fn typed() -> impl Iterator<Item = (&'static str, Hooks)> {
    REGISTRY
        .iter()
        .filter_map(|registration| match &registration.role {
            Role::Message(hooks) => Some((registration.proto, *hooks)),
            _ => None,
        })
}

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
pub(crate) fn absorbed() -> Vec<&'static str> {
    collect(|role| matches!(role, Role::Absorbed))
}

/// The [`Role::Unexposed`] proto names, sorted and de-duplicated.
pub(crate) fn unexposed() -> Vec<&'static str> {
    collect(|role| matches!(role, Role::Unexposed))
}
