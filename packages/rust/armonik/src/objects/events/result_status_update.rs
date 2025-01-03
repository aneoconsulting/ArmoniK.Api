use super::super::ResultStatus;

use crate::api::v3;

/// Represents an update to the status of a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResultStatusUpdate {
    /// The result id.
    pub result_id: String,
    /// The result status.
    pub status: ResultStatus,
}

super::super::impl_convert!(
    struct ResultStatusUpdate = v3::events::event_subscription_response::ResultStatusUpdate {
        result_id,
        status = enum status,
    }
);
