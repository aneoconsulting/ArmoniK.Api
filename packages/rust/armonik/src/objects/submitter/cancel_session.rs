use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {
    pub session_id: String,
}

super::super::impl_convert!(
    struct Request = v3::Session {
        session_id = id,
    }
);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Response {}

super::super::impl_convert!(
    struct Response = v3::Empty {
    }
);
