use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl From<Configuration> for v3::Configuration {
    fn from(value: Configuration) -> Self {
        Self {
            data_chunk_max_size: value.data_chunk_max_size,
        }
    }
}

impl From<v3::Configuration> for Configuration {
    fn from(value: v3::Configuration) -> Self {
        Self {
            data_chunk_max_size: value.data_chunk_max_size,
        }
    }
}

super::impl_convert!(Configuration : Option<v3::Configuration>);
