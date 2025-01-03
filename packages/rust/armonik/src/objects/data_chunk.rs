use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DataChunk {
    Data(Vec<u8>),
    Complete,
}

impl Default for DataChunk {
    fn default() -> Self {
        Self::Data(Vec::new())
    }
}

impl From<DataChunk> for v3::DataChunk {
    fn from(value: DataChunk) -> Self {
        match value {
            DataChunk::Data(data) => Self {
                r#type: Some(v3::data_chunk::Type::Data(data)),
            },
            DataChunk::Complete => Self {
                r#type: Some(v3::data_chunk::Type::DataComplete(true)),
            },
        }
    }
}

impl From<v3::DataChunk> for DataChunk {
    fn from(value: v3::DataChunk) -> Self {
        match value.r#type {
            Some(v3::data_chunk::Type::Data(data)) => Self::Data(data),
            Some(v3::data_chunk::Type::DataComplete(_)) => Self::Complete,
            None => Default::default(),
        }
    }
}

super::impl_convert!(req DataChunk : v3::DataChunk);
