use super::super::TaskOptionField;

use crate::api::v3;

/// Represents every available field in a Task.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum SummaryField {
    /// Unspecified.
    Unspecified = 0,
    /// The task ID.
    #[default]
    TaskId = 16,
    /// The session ID.
    SessionId = 1,
    /// The owner pod ID.
    OwnerPodId = 9,
    /// The initial task ID. Set when a task is submitted independently of retries.
    InitialTaskId = 10,
    /// The task status.
    Status = 2,
    /// The task creation date.
    CreatedAt = 3,
    /// The task submission date.
    SubmittedAt = 11,
    /// The task start date.
    StartedAt = 4,
    /// The task end date.
    EndedAt = 5,
    /// The task duration. Between the creation date and the end date.
    CreationToEndDuration = 6,
    /// The task calculated duration. Between the start date and the end date.
    ProcessingToEndDuration = 7,
    /// The task calculated duration. Between the received date and the end date.
    ReceivedToEndDuration = 18,
    /// The pod TTL (Time To Live).
    PodTtl = 12,
    /// The hostname of the container running the task.
    PodHostname = 13,
    /// When the task is received by the agent.
    ReceivedAt = 14,
    /// When the task is acquired by the agent.
    AcquiredAt = 15,
    /// When the task is processed by the agent.
    ProcessedAt = 17,
    /// When task data are fetched by the agent.
    FetchedAt = 19,
    /// The error message. Only set if task have failed.
    Error = 8,
    /// The ID of the Result that is used as a payload for this task.
    PayloadId = 20,
    /// The ID of the Result that is used as a payload for this task.
    CreatedBy = 21,
}

impl From<i32> for SummaryField {
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
            18 => Self::ReceivedToEndDuration,
            12 => Self::PodTtl,
            13 => Self::PodHostname,
            14 => Self::ReceivedAt,
            15 => Self::AcquiredAt,
            17 => Self::ProcessedAt,
            19 => Self::FetchedAt,
            8 => Self::Error,
            20 => Self::PayloadId,
            21 => Self::CreatedBy,
            _ => Self::Unspecified,
        }
    }
}

impl From<SummaryField> for v3::tasks::TaskSummaryField {
    fn from(value: SummaryField) -> Self {
        Self {
            field: value as i32,
        }
    }
}

impl From<v3::tasks::TaskSummaryField> for SummaryField {
    fn from(value: v3::tasks::TaskSummaryField) -> Self {
        value.field.into()
    }
}

super::super::impl_convert!(req SummaryField : v3::tasks::TaskSummaryField);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    Summary(SummaryField),
    /// The task option field.
    Option(TaskOptionField),
    /// Represents a generic field in a task option.
    OptionGeneric(String),
}

impl Default for Field {
    fn default() -> Self {
        Self::Summary(Default::default())
    }
}

impl From<Field> for v3::tasks::TaskField {
    fn from(value: Field) -> Self {
        Self {
            field: Some(match value {
                Field::Summary(field) => {
                    v3::tasks::task_field::Field::TaskSummaryField(field.into())
                }
                Field::Option(field) => v3::tasks::task_field::Field::TaskOptionField(field.into()),
                Field::OptionGeneric(field) => {
                    v3::tasks::task_field::Field::TaskOptionGenericField(
                        v3::tasks::TaskOptionGenericField { field },
                    )
                }
            }),
        }
    }
}

impl From<v3::tasks::TaskField> for Field {
    fn from(value: v3::tasks::TaskField) -> Self {
        match value.field {
            Some(v3::tasks::task_field::Field::TaskSummaryField(field)) => {
                Self::Summary(field.into())
            }
            Some(v3::tasks::task_field::Field::TaskOptionField(field)) => {
                Self::Option(field.into())
            }
            Some(v3::tasks::task_field::Field::TaskOptionGenericField(field)) => {
                Self::OptionGeneric(field.field)
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Field : v3::tasks::TaskField);
