use super::super::{TaskOptions, TaskStatus};
use super::Output;

use crate::api::v3;

/// A summary task object.
///
/// It contains only a subset of the fields from the underlying task object.
/// Used when a list of tasks are returned.
#[derive(Debug, Clone, Default)]
pub struct Raw {
    /// The task ID.
    pub task_id: String,
    /// The session ID. A task have only one related session but a session have many tasks.
    pub session_id: String,
    /// The owner pod ID.
    pub owner_pod_id: String,
    /// The initial task ID. Set when a task is submitted independently of retries.
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

super::super::impl_convert!(
    struct Raw = v3::tasks::TaskDetailed {
        task_id = id,
        session_id,
        owner_pod_id,
        initial_task_id,
        parent_task_ids,
        data_dependencies,
        expected_output_ids,
        retry_of_ids,
        status = enum status,
        status_message,
        options = option options,
        created_at,
        submitted_at,
        received_at,
        acquired_at,
        fetched_at,
        started_at,
        processed_at,
        ended_at,
        creation_to_end_duration,
        processing_to_end_duration,
        received_to_end_duration,
        pod_ttl,
        output = option output,
        pod_hostname,
        payload_id,
        created_by,
    }
);
