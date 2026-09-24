//! The host's own types, and every codec over them that is not reached through the C ABI.
//!
//! Everything in `generated` comes from `ffi/poc/rust/gen/generate.py`. `gen/generate.py
//! --check` fails if what is committed here is not what the generator writes.

pub mod generated {
    pub mod build;
    pub mod core_native;
    /// The same codec rendered with the plan's unknown-field option set to RETAIN
    /// (FIX-PLAN WP5, owner position 6): unknown fields captured into `unknown_fields` and
    /// re-emitted after the known ones. `core_native` is the DROP rendering.
    pub mod core_native_retain;
    pub mod prost_impl;
    pub mod types;
}

pub use generated::build;
pub use generated::types::*;
