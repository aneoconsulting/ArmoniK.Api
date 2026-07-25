use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.ResultRequest")]
pub struct ResultRequest {
    #[armonik(rename = "session")]
    pub session_id: String,
    pub result_id: String,
}

super::impl_convert!(
    struct ResultRequest = v3::ResultRequest {
        session_id = session,
        result_id,
    }
);
