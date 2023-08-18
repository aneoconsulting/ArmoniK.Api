use crate::api::v3;

/// Request to list versions.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

impl From<Request> for v3::versions::ListVersionsRequest {
    fn from(_value: Request) -> Self {
        Self {}
    }
}

impl From<v3::versions::ListVersionsRequest> for Request {
    fn from(_value: v3::versions::ListVersionsRequest) -> Self {
        Self {}
    }
}

super::super::impl_convert!(Request : Option<v3::versions::ListVersionsRequest>);

/// Response to list versions.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {
    /// Version of ArmoniK.Core
    pub core: String,
    /// Version of ArmoniK.API
    pub api: String,
}

impl From<Response> for v3::versions::ListVersionsResponse {
    fn from(value: Response) -> Self {
        Self {
            core: value.core,
            api: value.api,
        }
    }
}

impl From<v3::versions::ListVersionsResponse> for Response {
    fn from(value: v3::versions::ListVersionsResponse) -> Self {
        Self {
            core: value.core,
            api: value.api,
        }
    }
}

super::super::impl_convert!(Response : Option<v3::versions::ListVersionsResponse>);
