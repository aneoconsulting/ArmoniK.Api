#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.session_status.SessionStatus")]
pub enum SessionStatus {
    /// Session is open and accepting tasks for execution.
    Running,
    /// Session is cancelled. No more tasks can be submitted.
    Cancelled,
    /// Session is paused. Tasks can be submitted but no more new tasks will be executed. Already running tasks will continue until they finish.
    Paused,
    /// Session is closed. No more tasks can be submitted and executed.
    Closed,
    /// Session is purged. No more tasks can be submitted and executed. Results data will be deleted.
    Purged,
    /// Session is deleted. No more tasks can be submitted and executed. Sessions, tasks and results metadata associated to the session will be deleted.
    Deleted,
    /// Unspecified (zero) or a status unknown to this crate version;
    /// round-trips losslessly.
    Other(OtherSessionStatus),
}
