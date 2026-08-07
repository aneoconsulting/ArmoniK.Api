#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.task_status.TaskStatus")]
pub enum TaskStatus {
    Creating,
    Submitted,
    Dispatched,
    Completed,
    Error,
    Timeout,
    Cancelling,
    Cancelled,
    Processing,
    Processed,
    Retried,
    Pending,
    Paused,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Other(OtherTaskStatus),
}

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

    /// The variants carry their proto values as discriminants, which is what the derived
    /// `PartialOrd`/`Ord` compare.
    #[test]
    fn ordering_follows_the_proto_values() {
        assert!(TaskStatus::Creating < TaskStatus::Submitted);
        assert!(TaskStatus::Submitted < TaskStatus::Paused);

        // The catch-all covers the zero value and the unknown ones, which sort before every named
        // value, and among themselves by the raw value.
        assert!(TaskStatus::UNSPECIFIED < TaskStatus::Creating);
        assert!(TaskStatus::from(999) < TaskStatus::Creating);
        assert!(TaskStatus::UNSPECIFIED < TaskStatus::from(999));

        let mut sorted: Vec<TaskStatus> = (0..=13).rev().map(TaskStatus::from).collect();
        sorted.sort();
        let values: Vec<i32> = sorted.into_iter().map(i32::from).collect();
        assert_eq!(values, (0..=13).collect::<Vec<i32>>());
    }

    #[test]
    fn unspecified_is_matchable() {
        match TaskStatus::from(0) {
            TaskStatus::UNSPECIFIED => {}
            other => panic!("expected UNSPECIFIED, got {other:?}"),
        }
    }
}
