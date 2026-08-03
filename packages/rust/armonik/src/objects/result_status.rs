#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.result_status.ResultStatus")]
pub enum ResultStatus {
    Created,
    Completed,
    Aborted,
    Deleted,
    /// Result was not found. Whether this is a temporary or definitive state depends on the reliability of the sender.
    #[armonik(rename = "RESULT_STATUS_NOTFOUND")]
    NotFound,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Other(OtherResultStatus),
}
