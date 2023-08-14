mod create_results;
mod create_results_metadata;
mod delete;
mod download;
mod field;
mod filter;
mod get_owner_task_id;
mod list;
mod result_raw;
mod service_configuration;
mod upload_result;

pub use create_results::{CreateResultsRequest, CreateResultsResponse};
pub use create_results_metadata::{CreateResultsMetadataRequest, CreateResultsMetadataResponse};
pub use delete::{DeleteResultsDataRequest, DeleteResultsDataResponse};
pub use download::DownloadResultDataRequest;
pub use field::ResultField;
pub use filter::{ResultFilterField, ResultFilters, ResultFiltersAnd};
pub use get_owner_task_id::{GetOwnerTaskIdRequest, GetOwnerTaskIdResponse};
pub use list::{ResultListRequest, ResultListResponse};
pub use result_raw::ResultRaw;
pub use service_configuration::ResultsServiceConfiguration;
pub use upload_result::UploadResultDataRequest;

pub type ResultSort = super::Sort<ResultField>;
