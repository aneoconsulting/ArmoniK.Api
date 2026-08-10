#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.TaskRequest")]
pub struct TaskRequest {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
    pub payload: bytes::Bytes,
    pub payload_name: String,
}
