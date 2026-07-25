use super::super::TaskStatus;

/// Represents the submission of a new task in ArmoniK.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.NewTask")]
pub struct NewTask {
    /// The task id.
    pub task_id: String,
    /// The payload id.
    pub payload_id: String,
    /// The task id before retry.
    pub origin_task_id: String,
    /// The task status.
    pub status: TaskStatus,
    /// The keys of the expected outputs
    pub expected_output_keys: Vec<String>,
    /// The keys of the data dependencies.
    pub data_dependencies: Vec<String>,
    /// The list of retried tasks from the first retry to the current.
    pub retry_of_ids: Vec<String>,
    /// The parent task IDs. A tasks can be a child of another task.
    pub parent_task_ids: Vec<String>,
}
