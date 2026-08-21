#[armonik_macros::message("armonik.api.grpc.v1.Configuration")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Configuration {
    pub data_chunk_max_size: i32,
}
