use ::bytes::Bytes;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.DataChunk")]
pub enum DataChunk {
    /// No member set, which a zero-length `Data` chunk is not.
    #[default]
    Invalid,
    /// A chunk of data; decoding borrows the receive buffer.
    Data(Bytes),
    /// Marker for the end of the data stream.
    #[armonik(rename = "data_complete", present)]
    Complete,
}
