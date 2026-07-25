use super::super::ResultStatus;

/// Result metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.ResultMetaData")]
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
