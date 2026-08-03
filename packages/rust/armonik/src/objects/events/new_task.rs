use super::super::TaskStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.NewTask")]
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
