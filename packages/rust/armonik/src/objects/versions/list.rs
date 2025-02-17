use crate::api::v3;

/// Request to list versions.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {}

super::super::impl_convert!(
    struct Request = v3::versions::ListVersionsRequest {}
);

/// Response to list versions.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// Version of ArmoniK.Core
    pub core: String,
    /// Version of ArmoniK.API
    pub api: String,
}

super::super::impl_convert!(
    struct Response = v3::versions::ListVersionsResponse {
        core,
        api,
    }
);
