use super::{NewResult, NewTask, ResultOwnerUpdate, ResultStatusUpdate, TaskStatusUpdate};

use crate::api::v3;

/// Represents an event update. Only one update will be sent per message.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum Update {
    /// Invalid update
    #[default]
    Invalid = 0,
    /// An update to the status of a task.
    TaskStatusUpdate(TaskStatusUpdate) = 2,
    /// An update to the status of a result.
    ResultStatusUpdate(ResultStatusUpdate) = 3,
    /// An update to the owner of a result.
    ResultOwnerUpdate(ResultOwnerUpdate) = 4,
    /// A new task in ArmoniK.
    NewTask(NewTask) = 5,
    /// A new result in ArmoniK.
    NewResult(NewResult) = 6,
}

impl From<Update> for Option<v3::events::event_subscription_response::Update> {
    fn from(value: Update) -> Self {
        match value {
            Update::Invalid => None,
            Update::TaskStatusUpdate(update) => Some(
                v3::events::event_subscription_response::Update::TaskStatusUpdate(update.into()),
            ),
            Update::ResultStatusUpdate(update) => Some(
                v3::events::event_subscription_response::Update::ResultStatusUpdate(update.into()),
            ),
            Update::ResultOwnerUpdate(update) => Some(
                v3::events::event_subscription_response::Update::ResultOwnerUpdate(update.into()),
            ),
            Update::NewTask(update) => Some(
                v3::events::event_subscription_response::Update::NewTask(update.into()),
            ),
            Update::NewResult(update) => Some(
                v3::events::event_subscription_response::Update::NewResult(update.into()),
            ),
        }
    }
}

impl From<Option<v3::events::event_subscription_response::Update>> for Update {
    fn from(value: Option<v3::events::event_subscription_response::Update>) -> Self {
        match value {
            Some(v3::events::event_subscription_response::Update::TaskStatusUpdate(update)) => {
                Self::TaskStatusUpdate(update.into())
            }
            Some(v3::events::event_subscription_response::Update::ResultStatusUpdate(update)) => {
                Self::ResultStatusUpdate(update.into())
            }
            Some(v3::events::event_subscription_response::Update::ResultOwnerUpdate(update)) => {
                Self::ResultOwnerUpdate(update.into())
            }
            Some(v3::events::event_subscription_response::Update::NewTask(update)) => {
                Self::NewTask(update.into())
            }
            Some(v3::events::event_subscription_response::Update::NewResult(update)) => {
                Self::NewResult(update.into())
            }
            None => Self::Invalid,
        }
    }
}
