use super::Update;

/// Request to subscribe to the event stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionRequest")]
pub struct Request {
    /// Id of the session that will be used to subscribe events for.
    pub session_id: String,
    /// Filter for task related events.
    #[armonik(rename = "tasks_filters")]
    pub task_filters: super::super::tasks::filter::Or,
    /// Filter for result related events.
    #[armonik(rename = "results_filters")]
    pub result_filters: super::super::results::filter::Or,
    /// Filter the type of events to return. Empty means all.
    pub returned_events: Vec<super::EventsEnum>,
}

/// Response containing the update event.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse")]
pub struct Response {
    ///  Id of the session that will be used to subscribe events for.
    pub session_id: String,
    /// Represents an event update. Only one update will be sent per message.
    pub update: Update,
}
