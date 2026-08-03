use super::Update;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionRequest")]
pub struct Request {
    pub session_id: String,
    #[armonik(rename = "tasks_filters")]
    pub task_filters: super::super::tasks::filter::Or,
    #[armonik(rename = "results_filters")]
    pub result_filters: super::super::results::filter::Or,
    pub returned_events: Vec<super::EventsEnum>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse")]
pub struct Response {
    pub session_id: String,
    pub update: Update,
}
