use super::{NewResult, NewTask, ResultOwnerUpdate, ResultStatusUpdate, TaskStatusUpdate};

#[armonik_macros::message("armonik.api.grpc.v1.events.EventSubscriptionResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(oneof = "update")]
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
