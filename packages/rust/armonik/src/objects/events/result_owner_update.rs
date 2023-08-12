use crate::api::v3;

/// Represents an update to the owner task id of a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultOwnerUpdate {
    /// The result id.
    pub result_id: String,
    /// The previous owner id.
    pub previous_owner_id: String,
    /// The current owner id.
    pub current_owner_id: String,
}

impl From<ResultOwnerUpdate> for v3::events::event_subscription_response::ResultOwnerUpdate {
    fn from(value: ResultOwnerUpdate) -> Self {
        Self {
            result_id: value.result_id,
            previous_owner_id: value.previous_owner_id,
            current_owner_id: value.current_owner_id,
        }
    }
}

impl From<v3::events::event_subscription_response::ResultOwnerUpdate> for ResultOwnerUpdate {
    fn from(value: v3::events::event_subscription_response::ResultOwnerUpdate) -> Self {
        Self {
            result_id: value.result_id,
            previous_owner_id: value.previous_owner_id,
            current_owner_id: value.current_owner_id,
        }
    }
}

super::super::impl_convert!(ResultOwnerUpdate : Option<v3::events::event_subscription_response::ResultOwnerUpdate>);
