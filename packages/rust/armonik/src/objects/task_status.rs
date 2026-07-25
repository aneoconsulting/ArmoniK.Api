use crate::api::v3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.task_status.TaskStatus")]
pub enum TaskStatus {
    /// Task is being created in database.
    Creating,
    /// Task is submitted to the queue.
    Submitted,
    /// Task is dispatched to a worker.
    Dispatched,
    /// Task is completed.
    Completed,
    /// Task is in error state.
    Error,
    /// Task is in timeout state.
    Timeout,
    /// Task is being cancelled.
    Cancelling,
    /// Task is cancelled.
    Cancelled,
    /// Task is being processed.
    Processing,
    /// Task is processed.
    Processed,
    /// Task is retried.
    Retried,
    /// Task is waiting for its dependencies before becoming executable.
    Pending,
    /// Task is paused and will not be executed until session is resumed.
    Paused,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Other(OtherTaskStatus),
}

impl From<TaskStatus> for v3::task_status::TaskStatus {
    fn from(value: TaskStatus) -> Self {
        Self::try_from(i32::from(value)).unwrap_or(Self::Unspecified)
    }
}

impl From<v3::task_status::TaskStatus> for TaskStatus {
    fn from(value: v3::task_status::TaskStatus) -> Self {
        Self::from(value as i32)
    }
}

super::impl_convert!(req TaskStatus : v3::task_status::TaskStatus);

#[cfg(test)]
mod tests {
    use super::TaskStatus;

    #[test]
    fn conversions_are_normalizing_and_lossless() {
        assert_eq!(TaskStatus::from(1), TaskStatus::Creating);
        assert_eq!(TaskStatus::from(13), TaskStatus::Paused);
        assert_eq!(TaskStatus::from(0), TaskStatus::UNSPECIFIED);
        assert_eq!(TaskStatus::default(), TaskStatus::UNSPECIFIED);

        // Unknown values are preserved and never shadow a known one.
        let unknown = TaskStatus::from(999);
        assert!(matches!(unknown, TaskStatus::Other(raw) if raw.value() == 999));
        assert_eq!(i32::from(unknown), 999);
        assert_ne!(unknown, TaskStatus::UNSPECIFIED);

        for value in 0..=13 {
            assert_eq!(i32::from(TaskStatus::from(value)), value);
        }
    }

    #[test]
    fn unspecified_is_matchable() {
        match TaskStatus::from(0) {
            TaskStatus::UNSPECIFIED => {}
            other => panic!("expected UNSPECIFIED, got {other:?}"),
        }
    }
}
