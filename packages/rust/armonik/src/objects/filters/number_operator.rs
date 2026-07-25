use crate::api::v3;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterNumberOperator")]
pub enum FilterNumberOperator {
    /// Is equal to the specified number.
    #[default]
    Equal,
    /// Is not equal to the specified number.
    NotEqual,
    /// Is less than the specified number.
    LessThan,
    /// Is less than or equal to the specified number.
    LessThanOrEqual,
    /// Is greater than or equal to specified number.
    GreaterThanOrEqual,
    /// Is greater than the specified number.
    GreaterThan,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterNumberOperator),
}

impl From<FilterNumberOperator> for v3::FilterNumberOperator {
    fn from(value: FilterNumberOperator) -> Self {
        Self::try_from(i32::from(value)).unwrap_or_default()
    }
}

impl From<v3::FilterNumberOperator> for FilterNumberOperator {
    fn from(value: v3::FilterNumberOperator) -> Self {
        Self::from(value as i32)
    }
}

super::super::impl_convert!(req FilterNumberOperator : v3::FilterNumberOperator);
