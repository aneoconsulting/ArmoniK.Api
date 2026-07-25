use super::super::ResultStatus;

/// Represents an update to the status of a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultStatusUpdate")]
pub struct ResultStatusUpdate {
    /// The result id.
    pub result_id: String,
    /// The result status.
    pub status: ResultStatus,
}
