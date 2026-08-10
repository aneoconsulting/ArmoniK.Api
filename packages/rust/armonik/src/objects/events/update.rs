use super::{NewResult, NewTask, ResultOwnerUpdate, ResultStatusUpdate, TaskStatusUpdate};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(
    message = "armonik.api.grpc.v1.events.EventSubscriptionResponse",
    oneof = "update"
)]
pub enum Update {
    /// Invalid update
    #[default]
    Invalid,
    TaskStatusUpdate(TaskStatusUpdate),
    ResultStatusUpdate(ResultStatusUpdate),
    ResultOwnerUpdate(ResultOwnerUpdate),
    NewTask(NewTask),
    NewResult(NewResult),
}
