#[armonik_macros::enumeration("armonik.api.grpc.v1.task_status.TaskStatus")]
#[derive(Debug, Clone, Copy)]
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
    Unknown(UnknownTaskStatus),
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
        assert!(matches!(unknown, TaskStatus::Unknown(raw) if raw.value() == 999));
        assert_eq!(i32::from(unknown), 999);
        assert_ne!(unknown, TaskStatus::UNSPECIFIED);

        for value in 0..=13 {
            assert_eq!(i32::from(TaskStatus::from(value)), value);
        }
    }

    /// Ordering is the proto values', for every variant including the catch-all: it is emitted in
    /// terms of `i32::from`, so where a value sorts does not depend on whether this crate version
    /// happens to name it.
    #[test]
    fn ordering_follows_the_proto_values() {
        assert!(TaskStatus::Creating < TaskStatus::Submitted);
        assert!(TaskStatus::Submitted < TaskStatus::Paused);

        // The zero value sorts first and an unknown one sorts by what it holds, next to the named
        // values rather than before all of them.
        assert!(TaskStatus::UNSPECIFIED < TaskStatus::Creating);
        assert!(TaskStatus::Paused < TaskStatus::from(999));
        assert!(TaskStatus::UNSPECIFIED < TaskStatus::from(999));

        let mut sorted: Vec<TaskStatus> = (0..=13).rev().map(TaskStatus::from).collect();
        sorted.sort();
        let values: Vec<i32> = sorted.into_iter().map(i32::from).collect();
        assert_eq!(values, (0..=13).collect::<Vec<i32>>());
    }

    /// Equality and hashing read the same proto value ordering does, so the two spellings of one
    /// value agree even though only one of them is reachable: `From<i32>` normalizes, and the
    /// payload's field is private, so no caller outside this module can build the other.
    #[test]
    fn equality_and_hashing_follow_the_proto_values() {
        use std::collections::HashSet;

        assert_eq!(TaskStatus::from(0), TaskStatus::UNSPECIFIED);
        assert_ne!(TaskStatus::from(999), TaskStatus::UNSPECIFIED);
        assert_ne!(TaskStatus::from(999), TaskStatus::from(998));

        let statuses: HashSet<TaskStatus> =
            (0..=13).chain(998..=999).map(TaskStatus::from).collect();
        assert_eq!(statuses.len(), 16);
        assert!(statuses.contains(&TaskStatus::Paused));
        assert!(statuses.contains(&TaskStatus::from(999)));
    }
}
