use ::bytes::Bytes;

use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.DataChunk", oneof = "type")]
pub enum DataChunk {
    /// A chunk of data; decoding borrows the receive buffer.
    Data(Bytes),
    /// Marker for the end of the data stream.
    #[armonik(rename = "data_complete", present)]
    Complete,
}

impl Default for DataChunk {
    fn default() -> Self {
        Self::Data(Bytes::new())
    }
}

impl From<DataChunk> for v3::DataChunk {
    fn from(value: DataChunk) -> Self {
        match value {
            DataChunk::Data(data) => Self {
                r#type: Some(v3::data_chunk::Type::Data(data.to_vec())),
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
            Some(v3::data_chunk::Type::Data(data)) => Self::Data(data.into()),
            Some(v3::data_chunk::Type::DataComplete(_)) => Self::Complete,
            None => Default::default(),
        }
    }
}

super::impl_convert!(req DataChunk : v3::DataChunk);
