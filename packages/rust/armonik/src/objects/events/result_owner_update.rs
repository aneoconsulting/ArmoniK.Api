#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultOwnerUpdate")]
pub struct ResultOwnerUpdate {
    pub result_id: String,
    pub previous_owner_id: String,
    pub current_owner_id: String,
}
