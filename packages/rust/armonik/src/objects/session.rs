use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Session {
    pub id: String,
}

impl From<Session> for v3::Session {
    fn from(value: Session) -> Self {
        Self { id: value.id }
    }
}

impl From<v3::Session> for Session {
    fn from(value: v3::Session) -> Self {
        Self { id: value.id }
    }
}
