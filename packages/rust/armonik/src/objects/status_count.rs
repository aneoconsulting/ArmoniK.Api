use crate::api::v3;

use super::TaskStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatusCount {
    pub status: TaskStatus,
    pub count: i32,
}

super::impl_convert!(
    struct StatusCount = v3::StatusCount {
        status = enum status,
        count,
    }
);