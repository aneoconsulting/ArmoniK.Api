//! What the arms have in common: the payload table, the manifest, and the reference bytes.

pub mod arms;
pub mod arms_m2;
pub mod arms_m3;
pub mod arms_rest;
/// ABI v1 section 7.1's PULL delivery family, as two arms.
pub mod pull;
pub mod generated {
    /// WP5 step 10: without `unknown-fields`, the no-unknown variant's binding.
    /// reset-on-entry (measurement experiment): the binding rendered for that core.
    #[cfg_attr(all(not(feature = "reset-on-entry"), not(feature = "unknown-fields")), path = "binding_nounk.rs")]
    #[cfg_attr(all(feature = "reset-on-entry", feature = "unknown-fields"), path = "binding_roe.rs")]
    #[cfg_attr(all(feature = "reset-on-entry", not(feature = "unknown-fields")), path = "binding_roe_nounk.rs")]
    pub mod binding;
    /// reset-on-entry: the default binding (explicit resets) beside it, on the same core,
    /// for the in-process comparison (`ak_measure_*_set_roe(ctx, 0)` on its contexts).
    #[cfg(feature = "reset-on-entry")]
    #[cfg_attr(feature = "unknown-fields", path = "binding.rs")]
    #[cfg_attr(not(feature = "unknown-fields"), path = "binding_nounk.rs")]
    pub mod binding_explicit;
}

/// The host links the codec, so it knows the symbol (ABI v1 section 6), and a missing one
/// is a load failure rather than a wrong value in a field.
pub fn abi_version() -> u32 {
    unsafe { ak_abi::ak_abi_version() }
}
/// D24: the FSM consumer's token scratch (`fsm_with_<root>`'s `toks`), one per thread, for
/// the arms whose decode signature carries none. Reused, so a decode allocates no new one.
pub fn with_toks<R>(f: impl FnOnce(&mut Vec<i64>) -> R) -> R {
    thread_local! { static T: std::cell::RefCell<Vec<i64>> = const { std::cell::RefCell::new(Vec::new()) }; }
    T.with(|t| f(&mut t.borrow_mut()))
}
/// D19: the simdutf UTF-16 transcoder and the UTF exports (bins tc16_diff, tc16_bench).
pub mod d19;
pub mod manifest;
/// A layout perturbation used by `gen/stability.sh`; empty in a normal build.
pub mod pad;
