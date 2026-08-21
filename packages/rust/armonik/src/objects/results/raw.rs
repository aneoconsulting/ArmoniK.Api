use super::super::ResultStatus;

#[armonik_macros::message("armonik.api.grpc.v1.results.ResultRaw")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Raw {
    pub session_id: String,
    pub name: String,
    pub owner_task_id: String,
    pub status: ResultStatus,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub created_at: Option<prost_types::Timestamp>,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub completed_at: Option<prost_types::Timestamp>,
    pub result_id: String,
    pub size: i64,
    pub created_by: String,
    pub opaque_id: bytes::Bytes,
    pub manual_deletion: bool,
}
