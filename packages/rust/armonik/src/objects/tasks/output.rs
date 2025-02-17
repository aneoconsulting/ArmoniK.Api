use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Represents the task output.
pub enum Output {
    /// To know if a task have failed or succeed.
    #[default]
    Success,
    /// The error message. Only set if task have failed.
    Error(String),
}

impl From<Output> for v3::tasks::task_detailed::Output {
    fn from(value: Output) -> Self {
        match value {
            Output::Success => Self {
                success: true,
                ..Default::default()
            },
            Output::Error(message) => Self {
                error: message,
                ..Default::default()
            },
        }
    }
}

impl From<v3::tasks::task_detailed::Output> for Output {
    fn from(value: v3::tasks::task_detailed::Output) -> Self {
        if value.success {
            Self::Success
        } else {
            Self::Error(value.error)
        }
    }
}

super::super::impl_convert!(req Output : v3::tasks::task_detailed::Output);
