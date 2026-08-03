//! ArmoniK objects related to the Tasks service

pub mod cancel;
pub mod count_status;
pub mod filter;
pub mod get;
pub mod get_result_ids;
pub mod list;
pub mod list_detailed;
pub mod submit;

#[doc(hidden)]
pub mod field;
#[doc(hidden)]
pub mod output;
#[doc(hidden)]
pub mod raw;
#[doc(hidden)]
pub mod summary;

pub use field::{Field, OtherSummaryField, SummaryField};
pub use output::Output;
pub use raw::{Raw, Raw as Task};
pub use summary::Summary;

#[armonik_macros::alias("armonik.api.grpc.v1.tasks.ListTasksRequest.Sort")]
pub type Sort = super::Sort<Field>;
