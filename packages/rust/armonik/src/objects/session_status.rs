#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(enum = "armonik.api.grpc.v1.session_status.SessionStatus")]
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
