use super::super::TaskStatus;

#[armonik_macros::message("armonik.api.grpc.v1.events.EventSubscriptionResponse.NewTask")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NewTask {
    pub task_id: String,
    pub payload_id: String,
    pub origin_task_id: String,
    pub status: TaskStatus,
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub retry_of_ids: Vec<String>,
    pub parent_task_ids: Vec<String>,
}
