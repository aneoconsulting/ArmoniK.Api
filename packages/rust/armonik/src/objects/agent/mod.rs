//! ArmoniK objects related to the Agent service

mod result_metadata;

pub mod create_results;
pub mod create_results_metadata;
pub mod create_tasks;
pub mod get_common_data;
pub mod get_direct_data;
pub mod get_resource_data;
pub mod notify_result_data;
pub mod submit_tasks;

pub use result_metadata::ResultMetaData;
