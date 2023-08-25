use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub communication_token: String,
    pub key: String,
}

super::super::impl_convert!(
    struct Request = v3::agent::DataRequest {
        communication_token,
        key,
    }
);

pub type Response = super::data::Data;
