use crate::api::v3;

/// List of versions
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Versions {
    /// Version of ArmoniK.Core
    pub core: String,
    /// Version of ArmoniK.API
    pub api: String,
}

impl From<Versions> for v3::versions::ListVersionsResponse {
    fn from(value: Versions) -> Self {
        Self {
            core: value.core,
            api: value.api,
        }
    }
}

impl From<v3::versions::ListVersionsResponse> for Versions {
    fn from(value: v3::versions::ListVersionsResponse) -> Self {
        Self {
            core: value.core,
            api: value.api,
        }
    }
}

super::impl_convert!(Versions : Option<v3::versions::ListVersionsResponse>);
