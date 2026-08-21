#[armonik_macros::message("armonik.api.grpc.v1.Empty")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

#[armonik_macros::message("armonik.api.grpc.v1.results.ResultsServiceConfigurationResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    pub data_chunk_max_size: i32,
}
