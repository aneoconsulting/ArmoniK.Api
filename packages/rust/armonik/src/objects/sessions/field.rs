use super::super::TaskOptionField;

use crate::api::v3;

/// Represents every available field in a session raw.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum RawField {
    /// Unspecified.
    Unspecified = 0,
    /// The session ID.
    #[default]
    SessionId = 1,
    /// The session status.
    Status = 2,
    /// Whether clients can submit tasks in the session.
    ClientSubmission = 8,
    /// Whether workers can submit tasks in the session.
    WorkerSubmission = 9,
    /// The partition IDs.
    PartitionIds = 3,
    /// The task options. In fact, these are used as default value in child tasks.
    Options = 4,
    /// The creation date.
    CreatedAt = 5,
    /// The cancellation date. Only set when status is 'cancelled'.
    CancelledAt = 6,
    /// The closure date. Only set when status is 'closed'.
    ClosedAt = 12,
    /// The purge date. Only set when status is 'purged'.
    PurgedAt = 10,
    /// The deletion date. Only set when status is 'deleted'.
    DeletedAt = 11,
    /// The duration. Only set when status is 'cancelled' and 'closed'.
    Duration = 7,
}

impl From<i32> for RawField {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::SessionId,
            2 => Self::Status,
            8 => Self::ClientSubmission,
            9 => Self::WorkerSubmission,
            3 => Self::PartitionIds,
            4 => Self::Options,
            5 => Self::CreatedAt,
            6 => Self::CancelledAt,
            12 => Self::ClosedAt,
            10 => Self::PurgedAt,
            11 => Self::DeletedAt,
            7 => Self::Duration,
            _ => Self::Unspecified,
        }
    }
}

impl From<RawField> for v3::sessions::SessionRawField {
    fn from(value: RawField) -> Self {
        Self {
            field: value as i32,
        }
    }
}

impl From<v3::sessions::SessionRawField> for RawField {
    fn from(value: v3::sessions::SessionRawField) -> Self {
        Self::from(value.field)
    }
}

super::super::impl_convert!(req RawField : v3::sessions::SessionRawField);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    Raw(RawField),
    /// The task option field.
    TaskOption(TaskOptionField),
    /// Represents a generic field in a task option.
    TaskOptionGeneric(String),
}

impl Default for Field {
    fn default() -> Self {
        Self::Raw(Default::default())
    }
}

impl From<Field> for v3::sessions::SessionField {
    fn from(value: Field) -> Self {
        Self {
            field: Some(match value {
                Field::Raw(field) => {
                    v3::sessions::session_field::Field::SessionRawField(field.into())
                }
                Field::TaskOption(field) => {
                    v3::sessions::session_field::Field::TaskOptionField(field.into())
                }
                Field::TaskOptionGeneric(field) => {
                    v3::sessions::session_field::Field::TaskOptionGenericField(
                        v3::sessions::TaskOptionGenericField { field },
                    )
                }
            }),
        }
    }
}

impl From<v3::sessions::SessionField> for Field {
    fn from(value: v3::sessions::SessionField) -> Self {
        match value.field {
            Some(v3::sessions::session_field::Field::SessionRawField(field)) => {
                Self::Raw(field.into())
            }
            Some(v3::sessions::session_field::Field::TaskOptionField(field)) => {
                Self::TaskOption(field.into())
            }
            Some(v3::sessions::session_field::Field::TaskOptionGenericField(field)) => {
                Self::TaskOptionGeneric(field.field)
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Field : v3::sessions::SessionField);
