use crate::api::v3;

#[derive(Debug, Clone, Default)]
pub struct TaskRequest {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub payload: Vec<u8>,
    pub payload_name: String,
}

super::impl_convert!(
    struct TaskRequest = v3::TaskRequest {
        expected_output_keys,
        data_dependencies,
        payload,
        payload_name,
    }
);
