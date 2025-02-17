use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Session {
    pub session_id: String,
}

super::impl_convert!(
    struct Session = v3::Session {
        session_id = id,
    }
);
