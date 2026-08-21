#[armonik_macros::message("armonik.api.grpc.v1.Empty")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request {}

pub type Response = super::super::Configuration;
