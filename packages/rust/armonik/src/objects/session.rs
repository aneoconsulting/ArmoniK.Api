use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Session {
    pub id: String,
}

super::impl_convert!(
    struct Session = v3::Session {
        id,
    }
);
