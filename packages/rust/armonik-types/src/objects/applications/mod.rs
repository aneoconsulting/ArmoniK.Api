//! ArmoniK objects related to the Applications service

pub mod filter;
pub mod list;

#[doc(hidden)]
pub mod field;
#[doc(hidden)]
pub mod raw;

pub use field::{Field, OtherField};
pub use raw::Raw;

#[armonik_macros::alias("armonik.api.grpc.v1.applications.ListApplicationsRequest.Sort")]
pub type Sort = super::SortMany<Field>;
