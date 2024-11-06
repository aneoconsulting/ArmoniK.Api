//! ArmoniK objects related to the Sessions service

pub mod cancel;
pub mod close;
pub mod create;
pub mod delete;
pub mod filter;
pub mod get;
pub mod list;
pub mod pause;
pub mod purge;
pub mod resume;
pub mod stop_submission;

mod field;
mod raw;

pub use field::{Field, RawField};
pub use raw::Raw;

pub type Sort = super::Sort<Field>;

super::super::impl_convert!(
    struct Sort = crate::api::v3::sessions::list_sessions_request::Sort {
        field = option field,
        direction = enum direction,
    }
);
