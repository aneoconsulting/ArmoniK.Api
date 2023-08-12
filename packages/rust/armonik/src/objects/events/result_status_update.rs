use super::super::ResultStatus;

use crate::api::v3;

/// Represents an update to the status of a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultStatusUpdate {
    /// The result id.
    pub result_id: String,
    /// The result status.
    pub status: ResultStatus,
}

impl From<ResultStatusUpdate> for v3::events::event_subscription_response::ResultStatusUpdate {
    fn from(value: ResultStatusUpdate) -> Self {
        Self {
            result_id: value.result_id,
            status: value.status as i32,
        }
    }
}

impl From<v3::events::event_subscription_response::ResultStatusUpdate> for ResultStatusUpdate {
    fn from(value: v3::events::event_subscription_response::ResultStatusUpdate) -> Self {
        Self {
            result_id: value.result_id,
            status: value.status.into(),
        }
    }
}

super::super::impl_convert!(ResultStatusUpdate : Option<v3::events::event_subscription_response::ResultStatusUpdate>);
