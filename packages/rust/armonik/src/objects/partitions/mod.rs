//! ArmoniK objects related to the Partitions service

pub mod filter;
pub mod get;
pub mod list;

#[doc(hidden)]
pub mod field;
#[doc(hidden)]
pub mod raw;

pub use field::{Field, UnknownField};
pub use raw::Raw;

#[armonik_macros::alias("armonik.api.grpc.v1.partitions.ListPartitionsRequest.Sort")]
pub type Sort = super::Sort<Field>;
