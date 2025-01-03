use super::Raw;

use crate::api::v3;

/// Request to get an result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Result id. Must fail when name is empty.
    pub id: String,
}

super::super::impl_convert!(
    struct Request = v3::results::GetResultRequest {
        id = result_id,
    }
);

/// Response to get an result.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// The result.
    pub result: Raw,
}

super::super::impl_convert!(
    struct Response = v3::results::GetResultResponse {
        result = option result,
    }
);
