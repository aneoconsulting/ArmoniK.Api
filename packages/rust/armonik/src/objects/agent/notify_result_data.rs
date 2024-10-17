use crate::api::v3;

/// The metadata to identify the result to update.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestItem {
    /// The session of the result.
    pub session_id: String,
    /// The ID of the result.
    pub result_id: String,
}

impl From<(String, String)> for RequestItem {
    fn from(value: (String, String)) -> Self {
        Self {
            session_id: value.0,
            result_id: value.1,
        }
    }
}

super::super::impl_convert!(struct RequestItem = v3::agent::notify_result_data_request::ResultIdentifier {
    session_id,
    result_id,
});

/// Request for notifying results data are available in files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The identifier of the result to which add data.
    pub result_ids: Vec<RequestItem>,
}

super::super::impl_convert!(struct Request = v3::agent::NotifyResultDataRequest {
    communication_token,
    list result_ids = list ids,
});

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The list of ResultMetaData results that were created.
    pub result_ids: Vec<String>,
}

super::super::impl_convert!(struct Response = v3::agent::NotifyResultDataResponse {
    list result_ids,
});
