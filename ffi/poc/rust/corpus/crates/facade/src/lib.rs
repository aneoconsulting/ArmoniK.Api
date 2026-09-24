//! The corpus facade: the host types for every message of the corpus's reader schema, the
//! core-native codec over them in both unknown-field renderings, and the projection
//! (ffi/corpus/CONTRACT.md section 3). Everything in `generated` is written by
//! `poc/rust/gen/generate.py` from the corpus plan.

pub mod generated {
    pub mod core_native;
    pub mod core_native_retain;
    pub mod project;
    pub mod types;
}

pub use generated::types::*;
