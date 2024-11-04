use super::super::{TaskOptions, TaskStatus};
use super::Output;

use crate::api::v3;

/// A summary task object.
///
/// It contains only a subset of the fields from the underlying task object.
/// Used when a list of tasks are returned.
#[derive(Debug, Clone, Default)]
pub struct Summary {
    /// The task ID.
    pub task_id: String,
    /// The session ID. A task have only one related session but a session have many tasks.
    pub session_id: String,
    /// The owner pod ID.
    pub owner_pod_id: String,
    /// The initial task ID. Set when a task is submitted independently of retries.
    pub initial_task_id: String,
    /// Count the parent task IDs. A tasks can be a child of another task.
    pub count_parent_task_ids: i64,
    /// Count the data dependencies. A task have data dependencies.
    pub count_data_dependencies: i64,
    /// Count the expected output IDs. A task have expected output IDs.
    pub count_expected_output_ids: i64,
    /// Count the retry of IDs. When a task fail, retry will use these set of IDs.
    pub count_retry_of_ids: i64,
    /// The task status.
    pub status: TaskStatus,
    /// The status message.
    pub status_message: String,
    /// The task options.
    pub options: TaskOptions,
    /// The task creation date
    pub created_at: Option<prost_types::Timestamp>,
    /// The task submission date.
    pub submitted_at: Option<prost_types::Timestamp>,
    /// When the task is received by the agent.
    pub received_at: Option<prost_types::Timestamp>,
    /// When the task is acquired by the agent.
    pub acquired_at: Option<prost_types::Timestamp>,
    /// Task data retrieval end date.
    pub fetched_at: Option<prost_types::Timestamp>,
    /// The task start date.
    pub started_at: Option<prost_types::Timestamp>,
    /// The end of task processing date.
    pub processed_at: Option<prost_types::Timestamp>,
    /// The task end date. Also used when task failed.
    pub ended_at: Option<prost_types::Timestamp>,
    /// The task duration. Between the creation date and the end date.
    pub creation_to_end_duration: Option<prost_types::Duration>,
    /// The task calculated duration. Between the start date and the end date.
    pub processing_to_end_duration: Option<prost_types::Duration>,
    /// The task calculated duration. Between the received date and the end date.
    pub received_to_end_duration: Option<prost_types::Duration>,
    /// The pod TTL (Time To Live).
    pub pod_ttl: Option<prost_types::Timestamp>,
    /// The task output.
    pub output: Output,
    /// The hostname of the container running the task.
    pub pod_hostname: String,
    /// The ID of the Result that is used as a payload for this task.
    pub payload_id: String,
    /// The ID of the Task that as submitted this task, empty if none.
    pub created_by: String,
}

impl From<Summary> for v3::tasks::TaskSummary {
    fn from(value: Summary) -> Self {
        Self {
            id: value.task_id,
            session_id: value.session_id,
            owner_pod_id: value.owner_pod_id,
            initial_task_id: value.initial_task_id,
            count_parent_task_ids: value.count_parent_task_ids,
            count_data_dependencies: value.count_data_dependencies,
            count_expected_output_ids: value.count_expected_output_ids,
            count_retry_of_ids: value.count_retry_of_ids,
            status: value.status as i32,
            status_message: value.status_message,
            options: Some(value.options.into()),
            created_at: value.created_at,
            submitted_at: value.submitted_at,
            received_at: value.received_at,
            acquired_at: value.acquired_at,
            fetched_at: value.fetched_at,
            started_at: value.started_at,
            processed_at: value.processed_at,
            ended_at: value.ended_at,
            creation_to_end_duration: value.creation_to_end_duration,
            processing_to_end_duration: value.processing_to_end_duration,
            received_to_end_duration: value.received_to_end_duration,
            pod_ttl: value.pod_ttl,
            error: match value.output {
                Output::Success => Default::default(),
                Output::Error(message) => message,
            },
            pod_hostname: value.pod_hostname,
            payload_id: value.payload_id,
            created_by: value.created_by,
        }
    }
}

impl From<v3::tasks::TaskSummary> for Summary {
    fn from(value: v3::tasks::TaskSummary) -> Self {
        Self {
            task_id: value.id,
            session_id: value.session_id,
            owner_pod_id: value.owner_pod_id,
            initial_task_id: value.initial_task_id,
            count_parent_task_ids: value.count_parent_task_ids,
            count_data_dependencies: value.count_data_dependencies,
            count_expected_output_ids: value.count_expected_output_ids,
            count_retry_of_ids: value.count_retry_of_ids,
            status: value.status.into(),
            status_message: value.status_message,
            options: value.options.map_or_else(Default::default, Into::into),
            created_at: value.created_at,
            submitted_at: value.submitted_at,
            received_at: value.received_at,
            acquired_at: value.acquired_at,
            fetched_at: value.fetched_at,
            started_at: value.started_at,
            processed_at: value.processed_at,
            ended_at: value.ended_at,
            creation_to_end_duration: value.creation_to_end_duration,
            processing_to_end_duration: value.processing_to_end_duration,
            received_to_end_duration: value.received_to_end_duration,
            pod_ttl: value.pod_ttl,
            output: if value.error.is_empty() {
                Output::Success
            } else {
                Output::Error(value.error)
            },
            pod_hostname: value.pod_hostname,
            payload_id: value.payload_id,
            created_by: value.created_by,
        }
    }
}

super::super::impl_convert!(req Summary : v3::tasks::TaskSummary);
