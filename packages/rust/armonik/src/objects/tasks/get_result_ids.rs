use std::collections::HashMap;

/// Request for getting result ids of tasks ids.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.GetResultIdsRequest")]
pub struct Request {
    /// The task IDs.
    #[armonik(rename = "task_id")]
    pub task_ids: Vec<String>,
}

/// Response for getting result ids of tasks ids.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.GetResultIdsResponse")]
pub struct Response {
    /// The task results.
    #[armonik(with = "crate::codec::adapters::PairMap<1, 2>")]
    pub task_results: HashMap<String, Vec<String>>,
}
