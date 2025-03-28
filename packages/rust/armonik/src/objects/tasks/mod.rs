//! ArmoniK objects related to the Tasks service

pub mod cancel;
pub mod count_status;
pub mod filter;
pub mod get;
pub mod get_result_ids;
pub mod list;
pub mod list_detailed;
pub mod submit;

mod field;
mod output;
mod raw;
mod summary;

pub use field::{Field, SummaryField};
pub use output::Output;
pub use raw::{Raw, Raw as Task};
pub use summary::Summary;

pub type Sort = super::Sort<Field>;

super::super::impl_convert!(
    struct Sort = crate::api::v3::tasks::list_tasks_request::Sort {
        field = option field,
        direction = enum direction,
    }
);
