#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.WaitRequest")]
pub struct Request {
    pub filter: super::TaskFilter,
    pub stop_on_first_task_error: bool,
    pub stop_on_first_task_cancellation: bool,
}

pub type Response = super::super::Count;
