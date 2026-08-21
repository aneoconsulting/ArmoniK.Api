#[armonik_macros::message("armonik.api.grpc.v1.versions.ListVersionsRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

#[armonik_macros::message("armonik.api.grpc.v1.versions.ListVersionsResponse")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// Version of ArmoniK.Core
    pub core: String,
    /// Version of ArmoniK.API
    pub api: String,
}
