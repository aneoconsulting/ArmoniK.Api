use crate::api::v3;

/// Request to subscribe to the event stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventSubscriptionRequest {
    /// Id of the session that will be used to subscribe events for.
    pub session_id: String,
}

impl From<EventSubscriptionRequest> for v3::events::EventSubscriptionRequest {
    fn from(value: EventSubscriptionRequest) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

impl From<v3::events::EventSubscriptionRequest> for EventSubscriptionRequest {
    fn from(value: v3::events::EventSubscriptionRequest) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

super::super::impl_convert!(EventSubscriptionRequest : Option<v3::events::EventSubscriptionRequest>);
