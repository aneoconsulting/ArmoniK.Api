use crate::api::v3;

/// Represents every available field in a session raw.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum SessionRawField {
    /// Unspecified.
    Unspecified = 0,
    #[default]
    SessionId = 1,
    Status = 2,
    PartitionIds = 3,
    Options = 4,
    CreatedAt = 5,
    CancelledAt = 6,
    Duration = 7,
}

impl From<i32> for SessionRawField {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::SessionId,
            2 => Self::Status,
            3 => Self::PartitionIds,
            4 => Self::Options,
            5 => Self::CreatedAt,
            6 => Self::CancelledAt,
            7 => Self::Duration,
            _ => Self::Unspecified,
        }
    }
}

impl From<SessionRawField> for v3::sessions::SessionRawField {
    fn from(value: SessionRawField) -> Self {
        Self {
            field: value as i32,
        }
    }
}

impl From<v3::sessions::SessionRawField> for SessionRawField {
    fn from(value: v3::sessions::SessionRawField) -> Self {
        Self::from(value.field)
    }
}

super::super::impl_convert!(SessionRawField : Option<v3::sessions::SessionRawField>);

/// Represents a field in a task option.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum TaskOptionField {
    /// Unspecified.
    #[default]
    Unspecified = 0,
    MaxDuration = 1,
    MaxRetries = 2,
    Priority = 3,
    PartitionId = 4,
    ApplicationName = 5,
    ApplicationVersion = 6,
    ApplicationNamespace = 7,
    ApplicationService = 8,
    ApplicationEngine = 9,
}

impl From<i32> for TaskOptionField {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::MaxDuration,
            2 => Self::MaxRetries,
            3 => Self::Priority,
            4 => Self::PartitionId,
            5 => Self::ApplicationName,
            6 => Self::ApplicationVersion,
            7 => Self::ApplicationNamespace,
            8 => Self::ApplicationService,
            9 => Self::ApplicationEngine,
            _ => Self::Unspecified,
        }
    }
}

impl From<TaskOptionField> for v3::sessions::TaskOptionField {
    fn from(value: TaskOptionField) -> Self {
        Self {
            field: value as i32,
        }
    }
}

impl From<v3::sessions::TaskOptionField> for TaskOptionField {
    fn from(value: v3::sessions::TaskOptionField) -> Self {
        value.field.into()
    }
}

super::super::impl_convert!(TaskOptionField : Option<v3::sessions::TaskOptionField>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SessionField {
    SessionRawField(SessionRawField),
    /// The task option field.
    TaskOptionField(TaskOptionField),
    /// Represents a generic field in a task option.
    TaskOptionGenericField(String),
}

impl Default for SessionField {
    fn default() -> Self {
        Self::SessionRawField(Default::default())
    }
}

impl From<SessionField> for v3::sessions::SessionField {
    fn from(value: SessionField) -> Self {
        Self {
            field: Some(match value {
                SessionField::SessionRawField(field) => {
                    v3::sessions::session_field::Field::SessionRawField(field.into())
                }
                SessionField::TaskOptionField(field) => {
                    v3::sessions::session_field::Field::TaskOptionField(field.into())
                }
                SessionField::TaskOptionGenericField(field) => {
                    v3::sessions::session_field::Field::TaskOptionGenericField(
                        v3::sessions::TaskOptionGenericField { field: field },
                    )
                }
            }),
        }
    }
}

impl From<v3::sessions::SessionField> for SessionField {
    fn from(value: v3::sessions::SessionField) -> Self {
        match value.field {
            Some(v3::sessions::session_field::Field::SessionRawField(field)) => {
                Self::SessionRawField(field.into())
            }
            Some(v3::sessions::session_field::Field::TaskOptionField(field)) => {
                Self::TaskOptionField(field.into())
            }
            Some(v3::sessions::session_field::Field::TaskOptionGenericField(field)) => {
                Self::TaskOptionGenericField(field.field)
            }
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(SessionField : Option<v3::sessions::SessionField>);
