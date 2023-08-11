mod configuration;
mod count;
mod data_chunk;
mod error;
mod filters;
mod init_keyed_data_stream;
mod init_task_request;
mod output;
mod result_request;
mod result_status;
mod session;
mod session_status;
mod sort;
mod status_count;
mod task_error;
mod task_id;
mod task_id_list;
mod task_id_with_status;
mod task_list;
mod task_options;
mod task_output_request;
mod task_request;
mod task_request_header;
mod task_status;

pub mod partitions;
pub mod versions;

pub use configuration::Configuration;
pub use count::Count;
pub use data_chunk::DataChunk;
pub use error::Error;
pub use filters::*;
pub use init_keyed_data_stream::InitKeyedDataStream;
pub use init_task_request::InitTaskRequest;
pub use output::Output;
pub use result_request::ResultRequest;
pub use result_status::ResultStatus;
pub use session::Session;
pub use session_status::SessionStatus;
pub use sort::{Sort, SortDirection};
pub use status_count::StatusCount;
pub use task_error::TaskError;
pub use task_id::TaskId;
pub use task_id_list::TaskIdList;
pub use task_id_with_status::TaskIdWithStatus;
pub use task_list::TaskList;
pub use task_options::TaskOptions;
pub use task_output_request::TaskOutputRequest;
pub use task_request::TaskRequest;
pub use task_request_header::TaskRequestHeader;
pub use task_status::TaskStatus;

macro_rules! impl_convert {
    ($A:ty : Option<$B:ty>) => {
        impl From<$A> for Option<$B> {
            fn from(value: $A) -> Self {
                Some(value.into())
            }
        }

        impl From<Option<$B>> for $A {
            fn from(value: Option<$B>) -> Self {
                value.map_or_else(Default::default, Into::into)
            }
        }
    };
}
pub(crate) use impl_convert;
