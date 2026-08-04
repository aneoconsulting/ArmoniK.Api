//! The C ABI `ArmoniK.Api.Client.Legacy` calls into. Local to that .NET project; never published.
//!
//! No business logic and no knowledge of ArmoniK's services lives here: this crate routes bytes
//! and configuration to `armonik-transport`, which does not know this ABI exists either. Every
//! `extern "C"` entry point catches its own panics (see [`error::guard`]) rather than letting one
//! unwind across the boundary, which is undefined behaviour.

#![allow(non_camel_case_types)]

pub mod buffer;
pub mod client;
pub mod config;
pub mod error;
