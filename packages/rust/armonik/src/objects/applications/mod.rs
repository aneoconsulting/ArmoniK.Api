//! ArmoniK objects related to the Applications service

pub mod filter;
pub mod list;

mod field;
mod raw;

pub use field::{Field, OtherField};
pub use raw::Raw;

pub type Sort = super::SortMany<Field>;
