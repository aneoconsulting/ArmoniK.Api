use crate::api::v3;

/// Request for notifying results data are available in files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The identifier of the session where all the results to be notified are.
    pub session_id: String,
    /// The identifiers of the results to be notified.
    pub result_ids: Vec<String>,
}

impl From<Request> for v3::agent::NotifyResultDataRequest {
    fn from(value: Request) -> Self {
        Self {
            ids: value
                .result_ids
                .into_iter()
                .map(
                    |result_id| v3::agent::notify_result_data_request::ResultIdentifier {
                        session_id: value.session_id.clone(),
                        result_id,
                    },
                )
                .collect(),
            communication_token: value.communication_token,
        }
    }
}

impl From<v3::agent::NotifyResultDataRequest> for Request {
    fn from(value: v3::agent::NotifyResultDataRequest) -> Self {
        let mut session_id = None;
        let result_ids = value
            .ids
            .into_iter()
            .map(|id| {
                if session_id.is_none() {
                    session_id = Some(id.session_id)
                }
                id.result_id
            })
            .collect();

        Self {
            communication_token: value.communication_token,
            session_id: session_id.unwrap_or_default(),
            result_ids,
        }
    }
}

super::super::impl_convert!(req Request: v3::agent::NotifyResultDataRequest);

/// Response for creating results without data.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// The list of ResultMetaData results that were created.
    pub result_ids: Vec<String>,
}

super::super::impl_convert!(struct Response = v3::agent::NotifyResultDataResponse {
    list result_ids,
});
