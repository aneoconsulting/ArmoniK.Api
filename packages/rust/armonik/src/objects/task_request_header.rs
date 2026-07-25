use crate::api::v3;

#[derive(Debug, Clone, Default, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskRequestHeader")]
pub struct TaskRequestHeader {
    pub expected_output_keys: Vec<String>,
    pub data_dependencies: Vec<String>,
}

super::impl_convert!(
    struct TaskRequestHeader = v3::TaskRequestHeader {
        expected_output_keys,
        data_dependencies,
    }
);
