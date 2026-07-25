use crate::api::v3;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterDurationOperator")]
pub enum FilterDurationOperator {
    /// Is equal to the specified duration.
    #[default]
    Equal,
    /// Is not equal to the specified duration.
    NotEqual,
    /// Is shorter than the specified duration.
    ShorterThan,
    /// Is shorter or equal to the specified duration.
    ShorterThanOrEqual,
    /// Is longer or equal to the specified duration.
    LongerThanOrEqual,
    /// Is longer than the specified duration.
    LongerThan,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterDurationOperator),
}

impl From<FilterDurationOperator> for v3::FilterDurationOperator {
    fn from(value: FilterDurationOperator) -> Self {
        Self::try_from(i32::from(value)).unwrap_or_default()
    }
}

impl From<v3::FilterDurationOperator> for FilterDurationOperator {
    fn from(value: v3::FilterDurationOperator) -> Self {
        Self::from(value as i32)
    }
}

super::super::impl_convert!(req FilterDurationOperator : v3::FilterDurationOperator);
