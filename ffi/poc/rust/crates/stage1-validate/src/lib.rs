//! The stage 1 payload builder, as a library.
//!
//! It is hand-written from the rules `ffi/schema/emit/payloads.py` states, deliberately not
//! transliterated from it and deliberately not generated. The `prost` arm builds its objects
//! through this, and every facade arm builds them through the GENERATED builder, so the two
//! sides of the byte comparison are constructed by two independent routes.
pub mod build;
