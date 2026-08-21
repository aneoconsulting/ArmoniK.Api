#[armonik_macros::enumeration("armonik.api.grpc.v1.FilterStatusOperator")]
#[derive(Debug, Clone, Copy, Default)]
pub enum FilterStatusOperator {
    /// Is equal to the specified status.
    #[default]
    Equal,
    /// Is not equal to the specified status.
    NotEqual,
    /// Unknown to this crate version; round-trips losslessly.
    Unknown(UnknownFilterStatusOperator),
}
