use ::bytes::Bytes;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.DataChunk")]
pub enum DataChunk {
    /// No chunk: neither member was set.
    ///
    /// The absence used to decode to an empty `Data`, which is a legitimate zero-length chunk, so
    /// a truncated stream read as a well-formed one.
    #[default]
    Invalid,
    /// A chunk of data; decoding borrows the receive buffer.
    Data(Bytes),
    /// Marker for the end of the data stream.
    #[armonik(rename = "data_complete", present)]
    Complete,
}
