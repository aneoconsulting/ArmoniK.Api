use super::Update;

use crate::api::v3;

/// Request to subscribe to the event stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// Id of the session that will be used to subscribe events for.
    pub session_id: String,
}

super::super::impl_convert!(
    struct Request = v3::events::EventSubscriptionRequest {
        session_id,
    }
);

/// Response containing the update event.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    ///  Id of the session that will be used to subscribe events for.
    pub session_id: String,
    /// Represents an event update. Only one update will be sent per message.
    pub update: Update,
}

super::super::impl_convert!(
    struct Response = v3::events::EventSubscriptionResponse {
        session_id,
        update = update,
    }
);
