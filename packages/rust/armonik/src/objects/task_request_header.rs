#[armonik_macros::message("armonik.api.grpc.v1.TaskRequestHeader")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskRequestHeader {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
}
