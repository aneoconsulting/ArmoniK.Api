use super::super::ResultStatus;

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ResultMetaData {
    pub session_id: String,
    pub result_id: String,
    pub name: String,
    pub status: ResultStatus,
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
