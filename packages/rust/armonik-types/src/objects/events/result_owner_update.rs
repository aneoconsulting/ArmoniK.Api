/// Represents an update to the owner task id of a result.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultOwnerUpdate")]
pub struct ResultOwnerUpdate {
    /// The result id.
    pub result_id: String,
    /// The previous owner id.
    pub previous_owner_id: String,
    /// The current owner id.
    pub current_owner_id: String,
}
