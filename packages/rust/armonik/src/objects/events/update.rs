use super::{NewResult, NewTask, ResultOwnerUpdate, ResultStatusUpdate, TaskStatusUpdate};

#[armonik_macros::oneof("armonik.api.grpc.v1.events.EventSubscriptionResponse.update")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
