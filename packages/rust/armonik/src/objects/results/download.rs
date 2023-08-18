use crate::api::v3;

/// Request for getting a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// The session of the result.
    pub session_id: String,
    /// The ID of the result.
    pub result_id: String,
}

impl From<Request> for v3::results::DownloadResultDataRequest {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_id,
        }
    }
}

impl From<v3::results::DownloadResultDataRequest> for Request {
    fn from(value: v3::results::DownloadResultDataRequest) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_id,
        }
    }
}

super::super::impl_convert!(Request : Option<v3::results::DownloadResultDataRequest>);

/// Response for getting a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// A chunk of data.
    pub data_chunk: Vec<u8>,
}

impl From<Response> for v3::results::DownloadResultDataResponse {
    fn from(value: Response) -> Self {
        Self {
            data_chunk: value.data_chunk,
        }
    }
}

impl From<v3::results::DownloadResultDataResponse> for Response {
    fn from(value: v3::results::DownloadResultDataResponse) -> Self {
        Self {
            data_chunk: value.data_chunk,
        }
    }
}

super::super::impl_convert!(Response : Option<v3::results::DownloadResultDataResponse>);
