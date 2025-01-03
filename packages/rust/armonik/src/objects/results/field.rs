use crate::api::v3;

/// Represents every available field in a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(i32)]
pub enum Field {
    /// Unspecified.
    Unspecified = 0,
    /// The session ID.
    SessionId = 1,
    /// The result name.
    Name = 2,
    /// The owner task ID.
    OwnerTaskId = 3,
    /// The result status.
    Status = 4,
    /// The result creation date.
    CreatedAt = 5,
    /// The result completion date.
    CompletedAt = 6,
    /// The result ID.
    #[default]
    ResultId = 7,
    /// The size of the result.
    Size = 8,
    /// The ID of the Task that as submitted this result.
    CreatedBy = 9,
}

impl From<i32> for Field {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::SessionId,
            2 => Self::Name,
            3 => Self::OwnerTaskId,
            4 => Self::Status,
            5 => Self::CreatedAt,
            6 => Self::CompletedAt,
            7 => Self::ResultId,
            8 => Self::Size,
            9 => Self::CreatedBy,
            _ => Self::Unspecified,
        }
    }
}

impl From<Field> for v3::results::ResultField {
    fn from(value: Field) -> Self {
        Self {
            field: Some(v3::results::result_field::Field::ResultRawField(
                v3::results::ResultRawField {
                    field: value as i32,
                },
            )),
        }
    }
}

impl From<v3::results::ResultField> for Field {
    fn from(value: v3::results::ResultField) -> Self {
        match value.field {
            Some(v3::results::result_field::Field::ResultRawField(field)) => field.field.into(),
            None => Self::Unspecified,
        }
    }
}

super::super::impl_convert!(req Field : v3::results::ResultField);
