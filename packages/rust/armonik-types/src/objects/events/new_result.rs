use super::super::ResultStatus;

/// Represents the submission of a new result in ArmoniK.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.NewResult")]
pub struct NewResult {
    /// The result id.
    pub result_id: String,
    /// The owner task id.
    pub owner_id: String,
    /// The result status.
    pub status: ResultStatus,
}
