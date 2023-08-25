use crate::api::v3;

/// Request for getting a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// The session of the result.
    pub session_id: String,
    /// The ID of the result.
    pub result_id: String,
}

super::super::impl_convert!(
    struct Request = v3::results::DownloadResultDataRequest {
        session_id,
        result_id,
    }
);

/// Response for getting a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// A chunk of data.
    pub data_chunk: Vec<u8>,
}

super::super::impl_convert!(
    struct Response = v3::results::DownloadResultDataResponse {
        data_chunk,
    }
);
