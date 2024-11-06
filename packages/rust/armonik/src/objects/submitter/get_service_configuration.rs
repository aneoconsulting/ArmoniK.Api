use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

super::super::impl_convert!(
    struct Request = v3::Empty {}
);

pub type Response = super::super::Configuration;
