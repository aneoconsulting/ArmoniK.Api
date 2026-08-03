use super::super::{TaskOptions, TaskStatus};
use super::Output;

/// A summary task object.
///
/// It contains only a subset of the fields from the underlying task object.
/// Used when a list of tasks are returned.
#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.TaskSummary")]
pub struct Summary {
    /// The task ID.
    #[armonik(rename = "id")]
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
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub created_at: Option<prost_types::Timestamp>,
    /// The task submission date.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub submitted_at: Option<prost_types::Timestamp>,
    /// When the task is received by the agent.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub received_at: Option<prost_types::Timestamp>,
    /// When the task is acquired by the agent.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub acquired_at: Option<prost_types::Timestamp>,
    /// Task data retrieval end date.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub fetched_at: Option<prost_types::Timestamp>,
    /// The task start date.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub started_at: Option<prost_types::Timestamp>,
    /// The end of task processing date.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub processed_at: Option<prost_types::Timestamp>,
    /// The task end date. Also used when task failed.
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub ended_at: Option<prost_types::Timestamp>,
    /// The task duration. Between the creation date and the end date.
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_option_duration"))]
    pub creation_to_end_duration: Option<prost_types::Duration>,
    /// The task calculated duration. Between the start date and the end date.
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_option_duration"))]
    pub processing_to_end_duration: Option<prost_types::Duration>,
    /// The task calculated duration. Between the received date and the end date.
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_option_duration"))]
    pub received_to_end_duration: Option<prost_types::Duration>,
    /// The pod TTL (Time To Live).
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::utils::serde_option_timestamp")
    )]
    pub pod_ttl: Option<prost_types::Timestamp>,
    /// The task output.
    #[armonik(rename = "error", with = "super::output::ErrorAdapter")]
    pub output: Output,
    /// The hostname of the container running the task.
    pub pod_hostname: String,
    /// The ID of the Result that is used as a payload for this task.
    pub payload_id: String,
    /// The ID of the Task that as submitted this task, empty if none.
    pub created_by: String,
}
