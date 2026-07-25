use crate::api::v3;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.sort_direction.SortDirection")]
pub enum SortDirection {
    /// Unspecified. Do not use.
    Unspecified,
    /// Ascending.
    #[default]
    Asc,
    /// Descending
    Desc,
    /// Unknown to this crate version.
    Other(OtherSortDirection),
}

impl From<SortDirection> for v3::sort_direction::SortDirection {
    fn from(value: SortDirection) -> Self {
        Self::try_from(i32::from(value)).unwrap_or(Self::Unspecified)
    }
}

impl From<v3::sort_direction::SortDirection> for SortDirection {
    fn from(value: v3::sort_direction::SortDirection) -> Self {
        Self::from(value as i32)
    }
}

super::impl_convert!(req SortDirection : v3::sort_direction::SortDirection);

/// Sort on a single field; stands for the per-service `Sort` messages, whose
/// concrete instantiations are validated by the differential harness.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(generic)]
pub struct Sort<T> {
    #[armonik(tag = 1)]
    pub field: T,
    #[armonik(tag = 2)]
    pub direction: SortDirection,
}

/// Sort on several fields; stands for the per-service `Sort` messages with
/// repeated fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(generic)]
pub struct SortMany<T> {
    #[armonik(tag = 1)]
    pub fields: Vec<T>,
    #[armonik(tag = 2)]
    pub direction: SortDirection,
}
