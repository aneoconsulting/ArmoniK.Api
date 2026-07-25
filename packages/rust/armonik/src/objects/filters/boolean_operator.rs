use crate::api::v3;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterBooleanOperator")]
pub enum FilterBooleanOperator {
    /// Is the same as the specified boolean.
    #[default]
    Is,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterBooleanOperator),
}

impl From<FilterBooleanOperator> for v3::FilterBooleanOperator {
    fn from(value: FilterBooleanOperator) -> Self {
        Self::try_from(i32::from(value)).unwrap_or_default()
    }
}

impl From<v3::FilterBooleanOperator> for FilterBooleanOperator {
    fn from(value: v3::FilterBooleanOperator) -> Self {
        Self::from(value as i32)
    }
}

super::super::impl_convert!(req FilterBooleanOperator : v3::FilterBooleanOperator);
