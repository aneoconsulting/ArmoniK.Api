use crate::api::v3;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterStatusOperator")]
pub enum FilterStatusOperator {
    /// Is equal to the specified status.
    #[default]
    Equal,
    /// Is not equal to the specified status.
    NotEqual,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterStatusOperator),
}

impl From<FilterStatusOperator> for v3::FilterStatusOperator {
    fn from(value: FilterStatusOperator) -> Self {
        Self::try_from(i32::from(value)).unwrap_or_default()
    }
}

impl From<v3::FilterStatusOperator> for FilterStatusOperator {
    fn from(value: v3::FilterStatusOperator) -> Self {
        Self::from(value as i32)
    }
}

super::super::impl_convert!(req FilterStatusOperator : v3::FilterStatusOperator);
