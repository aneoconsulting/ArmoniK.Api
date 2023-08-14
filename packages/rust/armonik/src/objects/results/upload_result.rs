use crate::api::v3;

/// The possible messages that constitute a UploadResultDataRequest
/// They should be sent in the following order:
/// - id
/// - data_chunk (stream can have multiple data_chunk messages that represent data divided in several parts)
///
/// Data chunk cannot exceed the size returned by the GetServiceConfiguration rpc method
#[derive(Debug, Clone)]
pub enum UploadResultDataRequest {
    /// The identifier of the result to which add data.
    Identifier {
        /// The session of the result.
        session: String,
        /// The ID of the result.
        result_id: String,
    },
    /// A chunk of data.
    DataChunk(Vec<u8>),
}

impl Default for UploadResultDataRequest {
    fn default() -> Self {
        Self::Identifier {
            session: Default::default(),
            result_id: Default::default(),
        }
    }
}

impl From<UploadResultDataRequest> for v3::results::UploadResultDataRequest {
    fn from(value: UploadResultDataRequest) -> Self {
        match value {
            UploadResultDataRequest::Identifier { session, result_id } => Self {
                r#type: Some(v3::results::upload_result_data_request::Type::Id(
                    v3::results::upload_result_data_request::ResultIdentifier {
                        session_id: session,
                        result_id,
                    },
                )),
            },
            UploadResultDataRequest::DataChunk(data) => Self {
                r#type: Some(v3::results::upload_result_data_request::Type::DataChunk(
                    data,
                )),
            },
        }
    }
}

impl From<v3::results::UploadResultDataRequest> for UploadResultDataRequest {
    fn from(value: v3::results::UploadResultDataRequest) -> Self {
        match value.r#type {
            Some(v3::results::upload_result_data_request::Type::Id(id)) => Self::Identifier {
                session: id.session_id,
                result_id: id.result_id,
            },
            Some(v3::results::upload_result_data_request::Type::DataChunk(data)) => {
                Self::DataChunk(data)
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(UploadResultDataRequest : Option<v3::results::UploadResultDataRequest>);
