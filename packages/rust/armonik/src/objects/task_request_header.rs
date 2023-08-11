use crate::api::v3;

#[derive(Debug, Clone, Default)]
pub struct TaskRequestHeader {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
}

impl From<TaskRequestHeader> for v3::TaskRequestHeader {
    fn from(value: TaskRequestHeader) -> Self {
        Self {
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
        }
    }
}

impl From<v3::TaskRequestHeader> for TaskRequestHeader {
    fn from(value: v3::TaskRequestHeader) -> Self {
        Self {
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
        }
    }
}

super::impl_convert!(TaskRequestHeader : Option<v3::TaskRequestHeader>);
