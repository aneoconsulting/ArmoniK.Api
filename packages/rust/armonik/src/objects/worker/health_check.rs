#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {}

impl From<Request> for crate::Empty {
    fn from(_: Request) -> Self {
        Self {}
    }
}

impl From<crate::Empty> for Request {
    fn from(_: crate::Empty) -> Self {
        Self {}
    }
}

/// Health status of the worker, standing for the whole `HealthCheckReply`
/// message (a transparent wrapper around its `ServingStatus` enum).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent, message = "armonik.api.grpc.v1.worker.HealthCheckReply")]
pub enum Response {
    #[default]
    Unknown,
    Serving,
    NotServing,
    /// A status unknown to this crate version.
    Other(OtherServingStatus),
}
