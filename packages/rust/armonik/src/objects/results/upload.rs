use super::Raw;

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
        /// The session of the result.
        session_id: String,
        /// The ID of the result.
        result_id: String,
    },
    /// A chunk of data.
    DataChunk(Vec<u8>),
}

impl Default for Request {
    fn default() -> Self {
        Self::Identifier {
            session_id: Default::default(),
            result_id: Default::default(),
        }
    }
}

impl From<Request> for v3::results::UploadResultDataRequest {
    fn from(value: Request) -> Self {
        match value {
            Request::Identifier {
                session_id: session,
                result_id,
            } => Self {
                r#type: Some(v3::results::upload_result_data_request::Type::Id(
                    v3::results::upload_result_data_request::ResultIdentifier {
                        session_id: session,
                        result_id,
                    },
                )),
            },
            Request::DataChunk(data) => Self {
                r#type: Some(v3::results::upload_result_data_request::Type::DataChunk(
                    data,
                )),
            },
        }
    }
}

impl From<v3::results::UploadResultDataRequest> for Request {
    fn from(value: v3::results::UploadResultDataRequest) -> Self {
        match value.r#type {
            Some(v3::results::upload_result_data_request::Type::Id(id)) => Self::Identifier {
                session_id: id.session_id,
                result_id: id.result_id,
            },
            Some(v3::results::upload_result_data_request::Type::DataChunk(data)) => {
                Self::DataChunk(data)
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Request : v3::results::UploadResultDataRequest);

#[derive(Debug, Clone, Default)]
pub struct Response {
    pub result: Raw,
}

super::super::impl_convert!(
    struct Response = v3::results::UploadResultDataResponse {
        result = option result,
    }
);
