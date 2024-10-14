use crate::api::v3;

/// Request for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// Id of the result that will be retrieved.
    pub result_id: String,
}

super::super::impl_convert!(struct Request = v3::agent::DataRequest {
    communication_token,
    result_id,
});

/// Response for creating results without data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    /// Id of the result that will be retrieved.
    pub result_id: String,
}

super::super::impl_convert!(struct Response = v3::agent::DataResponse {
    result_id,
});
