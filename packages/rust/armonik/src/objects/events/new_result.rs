use super::super::ResultStatus;

#[armonik_macros::message("armonik.api.grpc.v1.events.EventSubscriptionResponse.NewResult")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NewResult {
    pub result_id: String,
    pub owner_id: String,
    pub status: ResultStatus,
}
