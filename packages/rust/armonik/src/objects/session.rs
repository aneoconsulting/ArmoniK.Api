use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Session {
    pub session_id: String,
}

super::impl_convert!(
    struct Session = v3::Session {
        session_id = id,
    }
);
