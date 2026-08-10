#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.applications.ApplicationRaw")]
pub struct Raw {
    pub name: String,
    pub version: String,
    /// Application namespace used in the executed class.
    pub namespace: String,
    /// Application service used in the executed class.
    pub service: String,
}
