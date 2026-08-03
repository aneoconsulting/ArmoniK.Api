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
