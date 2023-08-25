use crate::api::v3;

/// The possible messages that constitute a UploadResultDataRequest
/// They should be sent in the following order:
/// - id
/// - data_chunk (stream can have multiple data_chunk messages that represent data divided in several parts)
///
/// Data chunk cannot exceed the size returned by the GetServiceConfiguration rpc method
#[derive(Debug, Clone)]
pub enum Request {
    /// The identifier of the result to which add data.
    Identifier {
        /// Communication token received by the worker during task processing.
        communication_token: String,
        /// The session of the result.
        session: String,
        /// The ID of the result.
        result_id: String,
    },
    /// A chunk of data.
    DataChunk {
        /// Communication token received by the worker during task processing.
        communication_token: String,
        /// A chunk of data.
        chunk: Vec<u8>,
    },
}

impl Default for Request {
    fn default() -> Self {
        Self::Identifier {
            communication_token: Default::default(),
            session: Default::default(),
            result_id: Default::default(),
        }
    }
}

impl From<Request> for v3::agent::UploadResultDataRequest {
    fn from(value: Request) -> Self {
        match value {
            Request::Identifier {
                communication_token,
                session,
                result_id,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::upload_result_data_request::Type::Id(
                    v3::agent::upload_result_data_request::ResultIdentifier {
                        session_id: session,
                        result_id,
                    },
                )),
            },
            Request::DataChunk {
                communication_token,
                chunk,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::upload_result_data_request::Type::DataChunk(
                    chunk,
                )),
            },
        }
    }
}

impl From<v3::agent::UploadResultDataRequest> for Request {
    fn from(value: v3::agent::UploadResultDataRequest) -> Self {
        match value.r#type {
            Some(v3::agent::upload_result_data_request::Type::Id(id)) => Self::Identifier {
                communication_token: value.communication_token,
                session: id.session_id,
                result_id: id.result_id,
            },
            Some(v3::agent::upload_result_data_request::Type::DataChunk(chunk)) => {
                Self::DataChunk {
                    communication_token: value.communication_token,
                    chunk,
                }
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Request : v3::agent::UploadResultDataRequest);

/// Response for uploading data with stream for result.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// The Id of the result to which data were added.
    pub result_id: String,
}

super::super::impl_convert!(
    struct Response = v3::agent::UploadResultDataResponse {
        communication_token,
        result_id,
    }
);
