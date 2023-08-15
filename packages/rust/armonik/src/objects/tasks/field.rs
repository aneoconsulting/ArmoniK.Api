use super::super::TaskOptionField;

use crate::api::v3;

/// Represents every available field in a Task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum TaskSummaryField {
    /// Unspecified.
    Unspecified = 0,
    #[default]
    TaskId = 16,
    SessionId = 1,
    OwnerPodId = 9,
    InitialTaskId = 10,
    Status = 2,
    CreatedAt = 3,
    SubmittedAt = 11,
    StartedAt = 4,
    EndedAt = 5,
    CreationToEndDuration = 6,
    ProcessingToEndDuration = 7,
    PodTtl = 12,
    PodHostname = 13,
    ReceivedAt = 14,
    AcquiredAt = 15,
    Error = 8,
}

impl From<i32> for TaskSummaryField {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            16 => Self::TaskId,
            1 => Self::SessionId,
            9 => Self::OwnerPodId,
            10 => Self::InitialTaskId,
            2 => Self::Status,
            3 => Self::CreatedAt,
            11 => Self::SubmittedAt,
            4 => Self::StartedAt,
            5 => Self::EndedAt,
            6 => Self::CreationToEndDuration,
            7 => Self::ProcessingToEndDuration,
            12 => Self::PodTtl,
            13 => Self::PodHostname,
            14 => Self::ReceivedAt,
            15 => Self::AcquiredAt,
            8 => Self::Error,
            _ => Self::Unspecified,
        }
    }
}

impl From<TaskSummaryField> for v3::tasks::TaskSummaryField {
    fn from(value: TaskSummaryField) -> Self {
        Self {
            field: value as i32,
        }
    }
}

impl From<v3::tasks::TaskSummaryField> for TaskSummaryField {
    fn from(value: v3::tasks::TaskSummaryField) -> Self {
        value.field.into()
    }
}

super::super::impl_convert!(TaskSummaryField : Option<v3::tasks::TaskSummaryField>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskField {
    TaskSummaryField(TaskSummaryField),
    /// The task option field.
    TaskOptionField(TaskOptionField),
    /// Represents a generic field in a task option.
    TaskOptionGenericField(String),
}

impl Default for TaskField {
    fn default() -> Self {
        Self::TaskSummaryField(Default::default())
    }
}

impl From<TaskField> for v3::tasks::TaskField {
    fn from(value: TaskField) -> Self {
        Self {
            field: Some(match value {
                TaskField::TaskSummaryField(field) => {
                    v3::tasks::task_field::Field::TaskSummaryField(field.into())
                }
                TaskField::TaskOptionField(field) => {
                    v3::tasks::task_field::Field::TaskOptionField(field.into())
                }
                TaskField::TaskOptionGenericField(field) => {
                    v3::tasks::task_field::Field::TaskOptionGenericField(
                        v3::tasks::TaskOptionGenericField { field },
                    )
                }
            }),
        }
    }
}

impl From<v3::tasks::TaskField> for TaskField {
    fn from(value: v3::tasks::TaskField) -> Self {
        match value.field {
            Some(v3::tasks::task_field::Field::TaskSummaryField(field)) => {
                Self::TaskSummaryField(field.into())
            }
            Some(v3::tasks::task_field::Field::TaskOptionField(field)) => {
                Self::TaskOptionField(field.into())
            }
            Some(v3::tasks::task_field::Field::TaskOptionGenericField(field)) => {
                Self::TaskOptionGenericField(field.field)
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(TaskField : Option<v3::tasks::TaskField>);
