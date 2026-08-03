/// A raw application object.
///
/// Used when a list of applications is requested.
#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.applications.ApplicationRaw")]
pub struct Raw {
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Application namespace used in the executed class.
    pub namespace: String,
    /// Application service used in the executed class.
    pub service: String,
}
