use super::Update;

use crate::api::v3;

/// Request to subscribe to the event stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    /// Id of the session that will be used to subscribe events for.
    pub session_id: String,
}

impl From<Request> for v3::events::EventSubscriptionRequest {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

impl From<v3::events::EventSubscriptionRequest> for Request {
    fn from(value: v3::events::EventSubscriptionRequest) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

super::super::impl_convert!(Request : Option<v3::events::EventSubscriptionRequest>);

/// Response containing the update event.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    ///  Id of the session that will be used to subscribe events for.
    pub session_id: String,
    /// Represents an event update. Only one update will be sent per message.
    pub update: Update,
}

impl From<Response> for v3::events::EventSubscriptionResponse {
    fn from(value: Response) -> Self {
        Self {
            session_id: value.session_id,
            update: value.update.into(),
        }
    }
}

impl From<v3::events::EventSubscriptionResponse> for Response {
    fn from(value: v3::events::EventSubscriptionResponse) -> Self {
        Self {
            session_id: value.session_id,
            update: value.update.into(),
        }
    }
}

super::super::impl_convert!(Response : Option<v3::events::EventSubscriptionResponse>);
