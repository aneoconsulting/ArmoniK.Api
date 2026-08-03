#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultOwnerUpdate")]
pub struct ResultOwnerUpdate {
    pub result_id: String,
    pub previous_owner_id: String,
    pub current_owner_id: String,
}
