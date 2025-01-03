use super::Update;

use crate::{api::v3, utils::IntoCollection};

/// Request to subscribe to the event stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Id of the session that will be used to subscribe events for.
    pub session_id: String,
    /// Filter for task related events.
    pub task_filters: super::super::tasks::filter::Or,
    /// Filter for result related events.
    pub result_filters: super::super::results::filter::Or,
    /// Filter the type of events to return. Empty means all.
    pub returned_events: Vec<super::EventsEnum>,
}

impl From<Request> for v3::events::EventSubscriptionRequest {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.session_id,
            tasks_filters: Some(value.task_filters.into()),
            results_filters: Some(value.result_filters.into()),
            returned_events: value
                .returned_events
                .into_iter()
                .map(|v| v as i32)
                .collect(),
        }
    }
}

impl From<v3::events::EventSubscriptionRequest> for Request {
    fn from(value: v3::events::EventSubscriptionRequest) -> Self {
        Self {
            session_id: value.session_id,
            task_filters: value.tasks_filters.unwrap_or_default().into(),
            result_filters: value.results_filters.unwrap_or_default().into(),
            returned_events: value.returned_events.into_collect(),
        }
    }
}

super::super::impl_convert!(
    req Request : v3::events::EventSubscriptionRequest
);

/// Response containing the update event.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
