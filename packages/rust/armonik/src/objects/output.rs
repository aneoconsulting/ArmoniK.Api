use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Output {
    #[default]
    Ok,
    Error {
        details: String,
    },
}

impl From<Output> for v3::Output {
    fn from(value: Output) -> Self {
        match value {
            Output::Ok => v3::Output {
                r#type: Some(v3::output::Type::Ok(v3::Empty {})),
            },
            Output::Error { details } => v3::Output {
                r#type: Some(v3::output::Type::Error(v3::output::Error { details })),
            },
        }
    }
}

impl From<v3::Output> for Output {
    fn from(value: v3::Output) -> Self {
        match value.r#type {
            Some(v3::output::Type::Ok(_)) => Self::Ok,
            Some(v3::output::Type::Error(error)) => Self::Error {
                details: error.details,
            },
            None => Default::default(),
        }
    }
}

super::impl_convert!(req Output : v3::Output);
