#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
