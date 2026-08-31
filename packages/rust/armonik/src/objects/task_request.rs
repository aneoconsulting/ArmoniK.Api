#[armonik_macros::message("armonik.api.grpc.v1.TaskRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskRequest {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub payload: bytes::Bytes,
    pub payload_name: String,
}
