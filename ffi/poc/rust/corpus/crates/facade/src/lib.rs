//! The corpus facade: the host types for every message of the corpus's reader schema, the
//! core-native codec over them in both unknown-field renderings, and the projection
//! (ffi/corpus/CONTRACT.md section 3). Everything in `generated` is written by
//! `poc/rust/gen/generate.py` from the corpus plan.

pub mod generated {
    pub mod core_native;
    #[cfg(feature = "unknown-fields")]
    pub mod core_native_retain;
    pub mod project;
    // FIX-PLAN R-H22: the no-unknown build's facade has no `unknown_fields` member.
    #[cfg_attr(not(feature = "unknown-fields"), path = "types_nounk.rs")]
    pub mod types;
}

pub use generated::types::*;
