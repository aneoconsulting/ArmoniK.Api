use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum TaskStatus {
    #[default]
    Unspecified = 0, /* Task is in an unknown state. */
    Creating = 1,   /* Task is being created in database. */
    Submitted = 2,  /* Task is submitted to the queue. */
    Dispatched = 3, /* Task is dispatched to a worker. */
    Completed = 4,  /* Task is completed. */
    Error = 5,      /* Task is in error state. */
    Timeout = 6,    /* Task is in timeout state. */
    Cancelling = 7, /* Task is being cancelled. */
    Cancelled = 8,  /* Task is cancelled. */
    Processing = 9, /* Task is being processed. */
    Processed = 10, /* Task is processed. */
    Retried = 11,   /* Task is retried. */
}

impl From<i32> for TaskStatus {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Creating,
            2 => Self::Submitted,
            3 => Self::Dispatched,
            4 => Self::Completed,
            5 => Self::Error,
            6 => Self::Timeout,
            7 => Self::Cancelling,
            8 => Self::Cancelled,
            9 => Self::Processing,
            10 => Self::Processed,
            11 => Self::Retried,
            _ => Default::default(),
        }
    }
}

impl From<TaskStatus> for v3::task_status::TaskStatus {
    fn from(value: TaskStatus) -> Self {
        match value {
            TaskStatus::Unspecified => Self::Unspecified,
            TaskStatus::Creating => Self::Creating,
            TaskStatus::Submitted => Self::Submitted,
            TaskStatus::Dispatched => Self::Dispatched,
            TaskStatus::Completed => Self::Completed,
            TaskStatus::Error => Self::Error,
            TaskStatus::Timeout => Self::Timeout,
            TaskStatus::Cancelling => Self::Cancelling,
            TaskStatus::Cancelled => Self::Cancelled,
            TaskStatus::Processing => Self::Processing,
            TaskStatus::Processed => Self::Processed,
            TaskStatus::Retried => Self::Retried,
        }
    }
}

impl From<v3::task_status::TaskStatus> for TaskStatus {
    fn from(value: v3::task_status::TaskStatus) -> Self {
        match value {
            v3::task_status::TaskStatus::Unspecified => Self::Unspecified,
            v3::task_status::TaskStatus::Creating => Self::Creating,
            v3::task_status::TaskStatus::Submitted => Self::Submitted,
            v3::task_status::TaskStatus::Dispatched => Self::Dispatched,
            v3::task_status::TaskStatus::Completed => Self::Completed,
            v3::task_status::TaskStatus::Error => Self::Error,
            v3::task_status::TaskStatus::Timeout => Self::Timeout,
            v3::task_status::TaskStatus::Cancelling => Self::Cancelling,
            v3::task_status::TaskStatus::Cancelled => Self::Cancelled,
            v3::task_status::TaskStatus::Processing => Self::Processing,
            v3::task_status::TaskStatus::Processed => Self::Processed,
            v3::task_status::TaskStatus::Retried => Self::Retried,
        }
    }
}

super::impl_convert!(req TaskStatus : v3::task_status::TaskStatus);