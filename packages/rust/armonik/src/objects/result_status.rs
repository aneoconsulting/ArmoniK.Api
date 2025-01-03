use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(i32)]
pub enum ResultStatus {
    /// Result is in an unspecified state.
    #[default]
    Unspecified = 0,
    /// Result is created and task is created, submitted or dispatched.
    Created = 1,
    /// Result is completed with a completed task.
    Completed = 2,
    /// Result is aborted.
    Aborted = 3,
    /// Result is completed, but data has been deleted from object storage.
    Deleted = 4,
    /// Result was not found.
    NotFound = 127,
}

impl From<i32> for ResultStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Created,
            2 => Self::Completed,
            3 => Self::Aborted,
            4 => Self::Deleted,
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
            ResultStatus::Deleted => Self::Deleted,
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
            v3::result_status::ResultStatus::Deleted => Self::Deleted,
            v3::result_status::ResultStatus::Notfound => Self::NotFound,
        }
    }
}

super::impl_convert!(req ResultStatus : v3::result_status::ResultStatus);
