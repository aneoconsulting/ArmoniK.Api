use crate::api::v3;

use super::TaskStatus;

#[derive(Debug, Clone, Default, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Error")]
pub struct Error {
    pub task_status: TaskStatus,
    #[armonik(rename = "detail")]
    pub details: String,
}

super::impl_convert!(
    struct Error = v3::Error {
        task_status = enum task_status,
        details = detail,
    }
);
