use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {}

super::super::impl_convert!(
    struct Request = v3::Empty {
    }
);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Response {
    #[default]
    Unknown = 0,
    Serving = 1,
    NotServing = 2,
}

impl From<i32> for Response {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Serving,
            2 => Self::NotServing,
            _ => Self::Unknown,
        }
    }
}

impl From<Response> for v3::worker::HealthCheckReply {
    fn from(value: Response) -> Self {
        Self {
            status: value as i32,
        }
    }
}

impl From<v3::worker::HealthCheckReply> for Response {
    fn from(value: v3::worker::HealthCheckReply) -> Self {
        value.status.into()
    }
}

super::super::impl_convert!(req Response : v3::worker::HealthCheckReply);
