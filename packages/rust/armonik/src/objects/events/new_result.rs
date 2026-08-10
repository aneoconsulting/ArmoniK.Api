use super::super::ResultStatus;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.events.EventSubscriptionResponse.NewResult")]
pub struct NewResult {
    pub result_id: String,
    pub owner_id: String,
    pub status: ResultStatus,
}
