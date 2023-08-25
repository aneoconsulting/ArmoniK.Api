use crate::api::v3;

/// A raw application object.
///
/// Used when a list of applications is requested.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Raw {
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Application namespace used in the excecuted class.
    pub namespace: String,
    /// Application service used in the excecuted class.
    pub service: String,
}

super::super::impl_convert!(
    struct Raw = v3::applications::ApplicationRaw {
        name,
        version,
        namespace,
        service,
    }
);
