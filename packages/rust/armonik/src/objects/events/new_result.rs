use super::super::ResultStatus;

use crate::api::v3;

/// Represents an update to the status of a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NewResult {
    /// The result id.
    pub result_id: String,
    /// The owner task id.
    pub owner_id: String,
    /// The result status.
    pub status: ResultStatus,
}

impl From<NewResult> for v3::events::event_subscription_response::NewResult {
    fn from(value: NewResult) -> Self {
        Self {
            result_id: value.result_id,
            owner_id: value.owner_id,
            status: value.status as i32,
        }
    }
}

impl From<v3::events::event_subscription_response::NewResult> for NewResult {
    fn from(value: v3::events::event_subscription_response::NewResult) -> Self {
        Self {
            result_id: value.result_id,
            owner_id: value.owner_id,
            status: value.status.into(),
        }
    }
}

super::super::impl_convert!(NewResult : Option<v3::events::event_subscription_response::NewResult>);
