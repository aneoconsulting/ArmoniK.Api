use crate::api::v3;

#[derive(Debug, Clone, Default)]
pub struct TaskRequest {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub payload: Vec<u8>,
    pub payload_name: String,
}

impl From<TaskRequest> for v3::TaskRequest {
    fn from(value: TaskRequest) -> Self {
        Self {
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
            payload: value.payload,
            payload_name: value.payload_name,
        }
    }
}

impl From<v3::TaskRequest> for TaskRequest {
    fn from(value: v3::TaskRequest) -> Self {
        Self {
            expected_output_keys: value.expected_output_keys,
            data_dependencies: value.data_dependencies,
            payload: value.payload,
            payload_name: value.payload_name,
        }
    }
}

super::impl_convert!(TaskRequest : Option<v3::TaskRequest>);
