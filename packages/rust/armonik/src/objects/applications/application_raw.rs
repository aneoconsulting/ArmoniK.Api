use crate::api::v3;

/// A raw application object.
///
/// Used when a list of applications is requested.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationRaw {
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Application namespace used in the excecuted class.
    pub namespace: String,
    /// Application service used in the excecuted class.
    pub service: String,
}

impl From<ApplicationRaw> for v3::applications::ApplicationRaw {
    fn from(value: ApplicationRaw) -> Self {
        Self {
            name: value.name,
            version: value.version,
            namespace: value.namespace,
            service: value.service,
        }
    }
}

impl From<v3::applications::ApplicationRaw> for ApplicationRaw {
    fn from(value: v3::applications::ApplicationRaw) -> Self {
        Self {
            name: value.name,
            version: value.version,
            namespace: value.namespace,
            service: value.service,
        }
    }
}

super::super::impl_convert!(ApplicationRaw : Option<v3::applications::ApplicationRaw>);
