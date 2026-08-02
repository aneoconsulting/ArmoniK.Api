use std::collections::HashMap;

use super::TaskStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Count")]
pub struct Count {
    /// Number of tasks per status, from the repeated `StatusCount` pairs
    /// (duplicate statuses collapse, last wins).
    #[armonik(with = "crate::codec::adapters::PairMap<1, 2>")]
    pub values: HashMap<TaskStatus, i32>,
}
