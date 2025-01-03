use crate::api::v3;

/// Represents an update to the owner task id of a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResultOwnerUpdate {
    /// The result id.
    pub result_id: String,
    /// The previous owner id.
    pub previous_owner_id: String,
    /// The current owner id.
    pub current_owner_id: String,
}

super::super::impl_convert!(
    struct ResultOwnerUpdate = v3::events::event_subscription_response::ResultOwnerUpdate {
        result_id,
        previous_owner_id,
        current_owner_id,
    }
);
