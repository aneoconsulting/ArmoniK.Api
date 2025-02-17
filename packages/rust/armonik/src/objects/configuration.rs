use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Configuration {
    pub data_chunk_max_size: i32,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            data_chunk_max_size: 80 * 1024,
        }
    }
}

super::impl_convert!(
    struct Configuration = v3::Configuration {
        data_chunk_max_size,
    }
);
