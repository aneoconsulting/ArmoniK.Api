#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Empty")]
pub struct Request {}

/// Health status of the worker, standing for the whole `HealthCheckReply`
/// message (a transparent wrapper around its `ServingStatus` enum).
#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, )]
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
