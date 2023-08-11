use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum ResultStatus {
    #[default]
    Unspecified = 0, /* Result is in an unspecified state. */
    Created = 1,    /* Result is created and task is created, submitted or dispatched. */
    Completed = 2,  /* Result is completed with a completed task. */
    Aborted = 3,    /* Result is aborted. */
    NotFound = 127, /* Result was not found. */
}

impl From<i32> for ResultStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Created,
            2 => Self::Completed,
            3 => Self::Aborted,
            127 => Self::NotFound,
            _ => Default::default(),
        }
    }
}

impl From<ResultStatus> for v3::result_status::ResultStatus {
    fn from(value: ResultStatus) -> Self {
        match value {
            ResultStatus::Unspecified => Self::Unspecified,
            ResultStatus::Created => Self::Created,
            ResultStatus::Completed => Self::Completed,
            ResultStatus::Aborted => Self::Aborted,
            ResultStatus::NotFound => Self::Notfound,
        }
    }
}

impl From<v3::result_status::ResultStatus> for ResultStatus {
    fn from(value: v3::result_status::ResultStatus) -> Self {
        match value {
            v3::result_status::ResultStatus::Unspecified => Self::Unspecified,
            v3::result_status::ResultStatus::Created => Self::Created,
            v3::result_status::ResultStatus::Completed => Self::Completed,
            v3::result_status::ResultStatus::Aborted => Self::Aborted,
            v3::result_status::ResultStatus::Notfound => Self::NotFound,
        }
    }
}
