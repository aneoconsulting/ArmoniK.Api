/// Request to retrieve data.
///
/// Shares its wire form (`DataRequest`) with the other data RPCs, which the
/// stubs express with [`super::get_common_data::Request`]; this type only
/// exists so that the calls stay distinguishable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    /// Communication token received by the worker during task processing.
    pub communication_token: String,
    /// Id of the result that will be retrieved.
    pub result_id: String,
}

impl From<Request> for super::get_common_data::Request {
    fn from(value: Request) -> Self {
        Self {
            communication_token: value.communication_token,
            result_id: value.result_id,
        }
    }
}

impl From<super::get_common_data::Request> for Request {
    fn from(value: super::get_common_data::Request) -> Self {
        Self {
            communication_token: value.communication_token,
            result_id: value.result_id,
        }
    }
}

/// Response when data is available in the shared folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Response {
    /// Id of the result that will be retrieved.
    pub result_id: String,
}

impl From<Response> for super::get_common_data::Response {
    fn from(value: Response) -> Self {
        Self {
            result_id: value.result_id,
        }
    }
}

impl From<super::get_common_data::Response> for Response {
    fn from(value: super::get_common_data::Response) -> Self {
        Self {
            result_id: value.result_id,
        }
    }
}
