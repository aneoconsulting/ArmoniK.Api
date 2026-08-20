use std::collections::HashMap;

use super::TaskStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.Count")]
pub struct Count {
    /// Number of tasks per status, from the repeated `StatusCount` pairs
    /// (duplicate statuses collapse, last wins).
    #[armonik(inlined)]
    pub values: HashMap<TaskStatus, i32>,
}
