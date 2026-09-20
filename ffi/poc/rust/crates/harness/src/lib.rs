//! What the arms have in common: the payload table, the manifest, and the reference bytes.

pub mod arms;
pub mod arms_m2;
pub mod arms_m3;
pub mod arms_rest;
/// ABI v1 section 7.1's PULL delivery family, as two arms.
pub mod pull;
pub mod generated {
    pub mod binding;
}

/// The host links the codec, so it knows the symbol (ABI v1 section 6), and a missing one
/// is a load failure rather than a wrong value in a field.
pub fn abi_version() -> u32 {
    unsafe { ak_abi::ak_abi_version() }
}
pub mod manifest;
/// A layout perturbation used by `gen/stability.sh`; empty in a normal build.
pub mod pad;
