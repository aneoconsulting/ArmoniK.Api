use super::super::ResultStatus;

#[armonik_macros::message(
    "armonik.api.grpc.v1.events.EventSubscriptionResponse.ResultStatusUpdate"
)]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultStatusUpdate {
    pub result_id: String,
    pub status: ResultStatus,
}
