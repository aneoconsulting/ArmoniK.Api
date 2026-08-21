use std::collections::HashMap;

use super::TaskStatus;

#[armonik_macros::message("armonik.api.grpc.v1.Count")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Count {
    /// Number of tasks per status, from the repeated `StatusCount` pairs
    /// (duplicate statuses collapse, last wins).
    pub values: HashMap<TaskStatus, i32>,
}
