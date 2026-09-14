#[armonik_macros::message("armonik.api.grpc.v1.Empty")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

/// Health status of the worker, standing for the whole `HealthCheckReply`
/// message (a transparent wrapper around its `ServingStatus` enum).
#[armonik_macros::enumeration("armonik.api.grpc.v1.worker.HealthCheckReply")]
#[derive(Debug, Clone, Copy)]
#[armonik(transparent)]
pub enum Response {
    Serving,
    NotServing,
    /// The proto `UNKNOWN` (zero) status, reachable as [`Response::UNSPECIFIED`],
    /// or a status unknown to this crate version.
    Unknown(UnknownServingStatus),
}
