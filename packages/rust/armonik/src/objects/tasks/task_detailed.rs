use super::super::{TaskOptions, TaskStatus};
use super::Output;

use crate::api::v3;

/// A summary task object.
///
/// It contains only a subset of the fields from the underlying task object.
/// Used when a list of tasks are returned.
#[derive(Debug, Clone, Default)]
pub struct TaskDetailed {
    /// The task ID.
    pub id: String,
    /// The session ID. A task have only one related session but a session have many tasks.
    pub session_id: String,
    /// The owner pod ID.
    pub owner_pod_id: String,
    /// The initial task ID. Set when a task is submitted independantly of retries.
    pub initial_task_id: String,
    /// The parent task IDs. A tasks can be a child of another task.
    pub parent_task_ids: Vec<String>,
    /// The data dependencies. A task have data dependencies.
    pub data_dependencies: Vec<String>,
    /// The expected output IDs. A task have expected output IDs.
    pub expected_output_ids: Vec<String>,
    /// The retry of IDs. When a task fail, retry will use these set of IDs.
    pub retry_of_ids: Vec<String>,
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
    /// The task start date.
    pub started_at: Option<prost_types::Timestamp>,
    /// The task end date. Also used when task failed.
    pub ended_at: Option<prost_types::Timestamp>,
    /// The task duration. Between the creation date and the end date.
    pub creation_to_end_duration: Option<prost_types::Duration>,
    /// The task calculated duration. Between the start date and the end date.
    pub processing_to_end_duration: Option<prost_types::Duration>,
    /// The pod TTL (Time To Live).
    pub pod_ttl: Option<prost_types::Timestamp>,
    /// The task output.
    pub output: Output,
    /// The hostname of the container running the task.
    pub pod_hostname: String,
    /// When the task is received by the agent.
    pub received_at: Option<prost_types::Timestamp>,
    /// When the task is acquired by the agent.
    pub acquired_at: Option<prost_types::Timestamp>,
}

impl From<TaskDetailed> for v3::tasks::TaskDetailed {
    fn from(value: TaskDetailed) -> Self {
        Self {
            id: value.id,
            session_id: value.session_id,
            owner_pod_id: value.owner_pod_id,
            initial_task_id: value.initial_task_id,
            parent_task_ids: value.parent_task_ids,
            data_dependencies: value.data_dependencies,
            expected_output_ids: value.expected_output_ids,
            retry_of_ids: value.retry_of_ids,
            status: value.status as i32,
            status_message: value.status_message,
            options: value.options.into(),
            created_at: value.created_at,
            submitted_at: value.submitted_at,
            started_at: value.started_at,
            ended_at: value.ended_at,
            creation_to_end_duration: value.creation_to_end_duration,
            processing_to_end_duration: value.processing_to_end_duration,
            pod_ttl: value.pod_ttl,
            output: value.output.into(),
            pod_hostname: value.pod_hostname,
            received_at: value.received_at,
            acquired_at: value.acquired_at,
        }
    }
}

impl From<v3::tasks::TaskDetailed> for TaskDetailed {
    fn from(value: v3::tasks::TaskDetailed) -> Self {
        Self {
            id: value.id,
            session_id: value.session_id,
            owner_pod_id: value.owner_pod_id,
            initial_task_id: value.initial_task_id,
            parent_task_ids: value.parent_task_ids,
            data_dependencies: value.data_dependencies,
            expected_output_ids: value.expected_output_ids,
            retry_of_ids: value.retry_of_ids,
            status: value.status.into(),
            status_message: value.status_message,
            options: value.options.into(),
            created_at: value.created_at,
            submitted_at: value.submitted_at,
            started_at: value.started_at,
            ended_at: value.ended_at,
            creation_to_end_duration: value.creation_to_end_duration,
            processing_to_end_duration: value.processing_to_end_duration,
            pod_ttl: value.pod_ttl,
            output: value.output.into(),
            pod_hostname: value.pod_hostname,
            received_at: value.received_at,
            acquired_at: value.acquired_at,
        }
    }
}

super::super::impl_convert!(TaskDetailed : Option<v3::tasks::TaskDetailed>);
