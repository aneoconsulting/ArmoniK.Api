//! The host's own types, and every codec over them that is not reached through the C ABI.
//!
//! Everything in `generated` comes from `ffi/poc/rust/gen/generate.py`. `gen/generate.py
//! --check` fails if what is committed here is not what the generator writes.

pub mod generated {
    pub mod build;
    pub mod core_native;
    pub mod prost_impl;
    pub mod types;
}

pub use generated::build;
pub use generated::types::*;
