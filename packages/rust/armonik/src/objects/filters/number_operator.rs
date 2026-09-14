#[armonik_macros::enumeration("armonik.api.grpc.v1.FilterNumberOperator")]
#[derive(Debug, Clone, Copy, Default)]
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
    Unknown(UnknownFilterNumberOperator),
}
