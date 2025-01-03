use std::collections::HashMap;

use crate::api::v3;

use super::TaskStatus;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Count {
    pub values: HashMap<TaskStatus, i32>,
}

impl From<Count> for v3::Count {
    fn from(value: Count) -> Self {
        Self {
            values: value
                .values
                .into_iter()
                .map(|(status, count)| v3::StatusCount {
                    status: status as i32,
                    count,
                })
                .collect(),
        }
    }
}

impl From<v3::Count> for Count {
    fn from(value: v3::Count) -> Self {
        Self {
            values: value
                .values
                .into_iter()
                .map(|sc| (sc.status.into(), sc.count))
                .collect(),
        }
    }
}

super::impl_convert!(req Count : v3::Count);
