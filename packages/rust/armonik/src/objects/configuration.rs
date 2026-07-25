#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Configuration")]
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
