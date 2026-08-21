#[armonik_macros::message("armonik.api.grpc.v1.applications.ApplicationRaw")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Raw {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub service: String,
}
