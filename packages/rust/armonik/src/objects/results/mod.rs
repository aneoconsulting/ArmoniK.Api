//! ArmoniK objects related to the Results service

pub mod create;
pub mod create_metadata;
pub mod delete_data;
pub mod download;
pub mod filter;
pub mod get;
pub mod get_owner_task_id;
pub mod get_service_configuration;
pub mod import;
pub mod list;
pub mod upload;

mod field;
mod raw;

pub use field::{Field, OtherField};
pub use raw::Raw;

pub type Sort = super::Sort<Field>;
