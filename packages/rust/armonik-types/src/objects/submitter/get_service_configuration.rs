#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Empty")]
#[armonik(replace(
    target = "armonik.api.grpc.v1.submitter.GetServiceConfigurationRequest",
    service = "Submitter",
    method = "GetServiceConfiguration",
    input,
))]
pub struct Request {}

pub type Response = super::super::Configuration;
