use crate::api::v3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.result_status.ResultStatus")]
pub enum ResultStatus {
    /// Result is created and task is created, submitted or dispatched.
    Created,
    /// Result is completed with a completed task.
    Completed,
    /// Result is aborted.
    Aborted,
    /// Result is completed, but data has been deleted from object storage.
    Deleted,
    /// Result was not found. Whether this is a temporary or definitive state depends on the reliability of the sender.
    #[armonik(rename = "RESULT_STATUS_NOTFOUND")]
    NotFound,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Other(OtherResultStatus),
}

impl From<ResultStatus> for v3::result_status::ResultStatus {
    fn from(value: ResultStatus) -> Self {
        Self::try_from(i32::from(value)).unwrap_or(Self::Unspecified)
    }
}

impl From<v3::result_status::ResultStatus> for ResultStatus {
    fn from(value: v3::result_status::ResultStatus) -> Self {
        Self::from(value as i32)
    }
}

super::impl_convert!(req ResultStatus : v3::result_status::ResultStatus);
