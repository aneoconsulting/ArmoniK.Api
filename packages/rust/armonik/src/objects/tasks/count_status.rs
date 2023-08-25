use super::{super::StatusCount, filter};

use crate::api::v3;

/// Request to get count from tasks by status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The filters.
    pub filters: filter::Or,
}

super::super::impl_convert!(
    struct Request = v3::tasks::CountTasksByStatusRequest {
        filters = option filters,
    }
);

/// Response to get count from tasks by status.
#[derive(Debug, Clone, Default)]
pub struct Response {
    /// Number of tasks by status. Expected to have only 1 objct by tasks status.
    pub status: Vec<StatusCount>,
}

super::super::impl_convert!(
    struct Response = v3::tasks::CountTasksByStatusResponse {
        list status,
    }
);
