use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResultRequest {
    pub session_id: String,
    pub result_id: String,
}

super::impl_convert!(
    struct ResultRequest = v3::ResultRequest {
        session_id = session,
        result_id,
    }
);
