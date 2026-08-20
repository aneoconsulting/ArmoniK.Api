#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.applications.ApplicationRaw")]
pub struct Raw {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub service: String,
}
