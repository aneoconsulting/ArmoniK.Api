use super::Update;

#[armonik_macros::message("armonik.api.grpc.v1.events.EventSubscriptionRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub session_id: String,
    #[armonik(rename = "tasks_filters")]
    pub task_filters: super::super::tasks::filter::Or,
    #[armonik(rename = "results_filters")]
    pub result_filters: super::super::results::filter::Or,
    pub returned_events: Vec<super::EventsEnum>,
}

#[armonik_macros::message("armonik.api.grpc.v1.events.EventSubscriptionResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub session_id: String,
    pub update: Update,
}
