use super::Update;

use crate::api::v3;

/// Response containing the update event.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventSubscriptionResponse {
    ///  Id of the session that will be used to subscribe events for.
    pub session_id: String,
    /// Represents an event update. Only one update will be sent per message.
    pub update: Update,
}

impl From<EventSubscriptionResponse> for v3::events::EventSubscriptionResponse {
    fn from(value: EventSubscriptionResponse) -> Self {
        Self {
            session_id: value.session_id,
            update: value.update.into(),
        }
    }
}

impl From<v3::events::EventSubscriptionResponse> for EventSubscriptionResponse {
    fn from(value: v3::events::EventSubscriptionResponse) -> Self {
        Self {
            session_id: value.session_id,
            update: value.update.into(),
        }
    }
}

super::super::impl_convert!(EventSubscriptionResponse : Option<v3::events::EventSubscriptionResponse>);
