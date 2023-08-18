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

super::super::impl_convert!(struct NewResult = v3::events::event_subscription_response::NewResult { result_id, owner_id, status = enum status });
