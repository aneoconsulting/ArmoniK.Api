use crate::api::v3;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
