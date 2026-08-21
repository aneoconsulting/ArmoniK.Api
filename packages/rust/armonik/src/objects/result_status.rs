#[armonik_macros::enumeration("armonik.api.grpc.v1.result_status.ResultStatus")]
#[derive(Debug, Clone, Copy)]
pub enum ResultStatus {
    Created,
    Completed,
    Aborted,
    Deleted,
    /// Result was not found. Whether that is temporary or definitive depends on the reliability of
    /// the sender.
    #[armonik(rename = "RESULT_STATUS_NOTFOUND")]
    NotFound,
    /// Unspecified (zero) or a status unknown to this crate version; round-trips losslessly.
    Unknown(UnknownResultStatus),
}
