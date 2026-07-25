use crate::api::v3;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterStringOperator")]
pub enum FilterStringOperator {
    /// Is equal to the specified string.
    #[default]
    Equal,
    /// Is not equal to the specified string.
    NotEqual,
    /// Contains the specified substring.
    Contains,
    /// Does not contain the specified substring.
    NotContains,
    /// Starts with the specified substring.
    StartsWith,
    /// Ends with the specified substring.
    EndsWith,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterStringOperator),
}

impl From<FilterStringOperator> for v3::FilterStringOperator {
    fn from(value: FilterStringOperator) -> Self {
        Self::try_from(i32::from(value)).unwrap_or_default()
    }
}

impl From<v3::FilterStringOperator> for FilterStringOperator {
    fn from(value: v3::FilterStringOperator) -> Self {
        Self::from(value as i32)
    }
}

super::super::impl_convert!(req FilterStringOperator : v3::FilterStringOperator);
