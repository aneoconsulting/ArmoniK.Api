use super::super::TaskOptionField;

use crate::api::v3;

/// Represents every available field in a session raw.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum RawField {
    /// Unspecified.
    Unspecified = 0,
    #[default]
    SessionId = 1,
    Status = 2,
    ClientSubmission = 8,
    WorkerSubmission = 9,
    PartitionIds = 3,
    Options = 4,
    CreatedAt = 5,
    CancelledAt = 6,
    ClosedAt = 12,
    PurgedAt = 10,
    DeletedAt = 11,
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
