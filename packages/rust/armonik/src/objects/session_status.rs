#[armonik_macros::enumeration("armonik.api.grpc.v1.session_status.SessionStatus")]
#[derive(Debug, Clone, Copy)]
pub enum SessionStatus {
    Running,
    Cancelled,
    Paused,
    Closed,
    Purged,
    Deleted,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Unknown(UnknownSessionStatus),
}
