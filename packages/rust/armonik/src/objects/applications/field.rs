use crate::api::v3;

/// Represents every available field in a Application.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum ApplicationField {
    /// Unspecified.
    Unspecified = 0,
    /// Application name.
    #[default]
    Name = 1,
    /// Application version.
    Version = 2,
    /// Application namespace.
    Namespace = 3,
    /// Application service.
    Service = 4,
}

impl From<i32> for ApplicationField {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Name,
            2 => Self::Version,
            3 => Self::Namespace,
            4 => Self::Service,
            _ => Self::Unspecified,
        }
    }
}

impl From<ApplicationField> for v3::applications::ApplicationField {
    fn from(value: ApplicationField) -> Self {
        Self {
            field: Some(
                v3::applications::application_field::Field::ApplicationField(
                    v3::applications::ApplicationRawField {
                        field: value as i32,
                    },
                ),
            ),
        }
    }
}

impl From<v3::applications::ApplicationField> for ApplicationField {
    fn from(value: v3::applications::ApplicationField) -> Self {
        match value.field {
            Some(v3::applications::application_field::Field::ApplicationField(field)) => {
                Self::from(field.field)
            }
            None => Self::Unspecified,
        }
    }
}

super::super::impl_convert!(ApplicationField : Option<v3::applications::ApplicationField>);
