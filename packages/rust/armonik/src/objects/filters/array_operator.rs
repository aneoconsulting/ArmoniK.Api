use crate::api::v3;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterArrayOperator")]
pub enum FilterArrayOperator {
    /// Contains the specified element.
    #[default]
    Contains,
    /// Does not contain the specified element.
    NotContains,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterArrayOperator),
}

impl From<FilterArrayOperator> for v3::FilterArrayOperator {
    fn from(value: FilterArrayOperator) -> Self {
        Self::try_from(i32::from(value)).unwrap_or_default()
    }
}

impl From<v3::FilterArrayOperator> for FilterArrayOperator {
    fn from(value: v3::FilterArrayOperator) -> Self {
        Self::from(value as i32)
    }
}

super::super::impl_convert!(req FilterArrayOperator : v3::FilterArrayOperator);
