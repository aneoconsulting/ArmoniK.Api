use super::super::ResultStatus;

use crate::api::v3;

/// Result metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResultMetaData {
    /// The session ID.
    pub session_id: String,
    /// The result ID.
    pub result_id: String,
    /// The result name.
    pub name: String,
    /// The result status.
    pub status: ResultStatus,
    /// The result creation date.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub created_at: Option<prost_types::Timestamp>,
}

super::super::impl_convert!(
    struct ResultMetaData = v3::agent::ResultMetaData {
        session_id,
        result_id,
        name,
        status = enum status,
        option created_at,
    }
);
