use super::{NewResult, NewTask, ResultOwnerUpdate, ResultStatusUpdate, TaskStatusUpdate};

/// Represents an event update. Only one update will be sent per message.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.events.EventSubscriptionResponse",
    oneof = "update"
)]
pub enum Update {
    /// Invalid update
    #[default]
    Invalid,
    /// An update to the status of a task.
    TaskStatusUpdate(TaskStatusUpdate),
    /// An update to the status of a result.
    ResultStatusUpdate(ResultStatusUpdate),
    /// An update to the owner of a result.
    ResultOwnerUpdate(ResultOwnerUpdate),
    /// A new task in ArmoniK.
    NewTask(NewTask),
    /// A new result in ArmoniK.
    NewResult(NewResult),
}
