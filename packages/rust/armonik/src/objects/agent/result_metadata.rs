use super::super::ResultStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[armonik(message = "armonik.api.grpc.v1.agent.ResultMetaData")]
pub struct ResultMetaData {
    pub session_id: String,
    pub result_id: String,
    pub name: String,
    pub status: ResultStatus,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub created_at: Option<prost_types::Timestamp>,
}
