use super::super::TaskStatus;

/// Task selector of the filter.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter", oneof = "ids")]
pub enum TaskFilterIds {
    /// No selector. Distinct from `Sessions([])`, which selects the tasks of no session.
    #[default]
    Invalid,
    /// Select the tasks from their session IDs.
    #[armonik(rename = "session", flatten)]
    Sessions(Vec<String>),
    /// Select the tasks from their task IDs.
    #[armonik(rename = "task", flatten)]
    Tasks(Vec<String>),
}

/// Status selector of the filter.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(
    message = "armonik.api.grpc.v1.submitter.TaskFilter",
    oneof = "statuses"
)]
pub enum TaskFilterStatuses {
    /// No selector. Distinct from `Exclude([])`, which is a constraint that happens to be vacuous.
    #[default]
    Invalid,
    /// Select the tasks whose status is one of these.
    #[armonik(rename = "included", flatten)]
    Include(Vec<TaskStatus>),
    /// Select the tasks whose status is none of these.
    #[armonik(rename = "excluded", flatten)]
    Exclude(Vec<TaskStatus>),
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter")]
pub struct TaskFilter {
    pub ids: TaskFilterIds,
    pub statuses: TaskFilterStatuses,
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::*;

    /// Independent prost-derived reference, with the two oneofs spelled out as optional fields.
    #[derive(Clone, PartialEq, Message)]
    struct RefFilter {
        #[prost(message, optional, tag = "1")]
        session: Option<RefIds>,
        #[prost(message, optional, tag = "3")]
        task: Option<RefIds>,
        #[prost(message, optional, tag = "4")]
        included: Option<RefStatuses>,
        #[prost(message, optional, tag = "5")]
        excluded: Option<RefStatuses>,
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefIds {
        #[prost(string, repeated, tag = "1")]
        ids: Vec<String>,
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefStatuses {
        #[prost(int32, repeated, tag = "1")]
        statuses: Vec<i32>,
    }

    fn statuses(status: TaskStatus) -> Option<RefStatuses> {
        Some(RefStatuses {
            statuses: vec![i32::from(status)],
        })
    }

    /// Each selector variant names its own proto member, in both directions.
    #[test]
    fn selectors_bind_their_own_proto_member() {
        for (ours, theirs) in [
            (
                TaskFilterStatuses::Include(vec![TaskStatus::Completed]),
                RefFilter {
                    included: statuses(TaskStatus::Completed),
                    ..Default::default()
                },
            ),
            (
                TaskFilterStatuses::Exclude(vec![TaskStatus::Cancelled]),
                RefFilter {
                    excluded: statuses(TaskStatus::Cancelled),
                    ..Default::default()
                },
            ),
        ] {
            let filter = TaskFilter {
                ids: TaskFilterIds::Tasks(vec![String::from("task-id")]),
                statuses: ours.clone(),
            };
            let expected = RefFilter {
                task: Some(RefIds {
                    ids: vec![String::from("task-id")],
                }),
                ..theirs.clone()
            };

            assert_eq!(
                RefFilter::decode(filter.encode_to_vec().as_slice()).unwrap(),
                expected
            );
            assert_eq!(
                TaskFilter::decode(expected.encode_to_vec().as_slice()).unwrap(),
                filter
            );

            // The id selector too, on the other oneof.
            let filter = TaskFilter {
                ids: TaskFilterIds::Sessions(vec![String::from("session-id")]),
                statuses: ours,
            };
            let expected = RefFilter {
                session: Some(RefIds {
                    ids: vec![String::from("session-id")],
                }),
                ..theirs
            };

            assert_eq!(
                RefFilter::decode(filter.encode_to_vec().as_slice()).unwrap(),
                expected
            );
            assert_eq!(
                TaskFilter::decode(expected.encode_to_vec().as_slice()).unwrap(),
                filter
            );
        }
    }
}
