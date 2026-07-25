use crate::api::v3;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterDateOperator")]
pub enum FilterDateOperator {
    /// Is equal to the specified date.
    #[default]
    Equal,
    /// Is not equal to the specified date.
    NotEqual,
    /// Is before the specified date.
    Before,
    /// Is before or equal to the specified date.
    BeforeOrEqual,
    /// Is After or equal to the specified date.
    AfterOrEqual,
    /// Is after the specified date.
    After,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterDateOperator),
}

impl From<FilterDateOperator> for v3::FilterDateOperator {
    fn from(value: FilterDateOperator) -> Self {
        Self::try_from(i32::from(value)).unwrap_or_default()
    }
}

impl From<v3::FilterDateOperator> for FilterDateOperator {
    fn from(value: v3::FilterDateOperator) -> Self {
        Self::from(value as i32)
    }
}

super::super::impl_convert!(req FilterDateOperator : v3::FilterDateOperator);
