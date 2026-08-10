#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.WaitRequest")]
pub struct Request {
    pub filter: super::TaskFilter,
    pub stop_on_first_task_error: bool,
    pub stop_on_first_task_cancellation: bool,
}

#[armonik_macros::reflect]
pub type Response = super::super::Count;
