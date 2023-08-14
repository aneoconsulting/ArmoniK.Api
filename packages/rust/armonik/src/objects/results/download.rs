use crate::api::v3;

/// Request for getting a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DownloadResultDataRequest {
    /// The session of the result.
    pub session_id: String,
    /// The ID of the result.
    pub result_id: String,
}

impl From<DownloadResultDataRequest> for v3::results::DownloadResultDataRequest {
    fn from(value: DownloadResultDataRequest) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_id,
        }
    }
}

impl From<v3::results::DownloadResultDataRequest> for DownloadResultDataRequest {
    fn from(value: v3::results::DownloadResultDataRequest) -> Self {
        Self {
            session_id: value.session_id,
            result_id: value.result_id,
        }
    }
}

super::super::impl_convert!(DownloadResultDataRequest : Option<v3::results::DownloadResultDataRequest>);
