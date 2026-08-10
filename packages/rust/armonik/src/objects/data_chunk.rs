use ::bytes::Bytes;

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.DataChunk")]
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
