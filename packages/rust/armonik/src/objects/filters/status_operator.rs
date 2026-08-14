#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, Default)]
#[armonik(enum = "armonik.api.grpc.v1.FilterStatusOperator")]
pub enum FilterStatusOperator {
    /// Is equal to the specified status.
    #[default]
    Equal,
    /// Is not equal to the specified status.
    NotEqual,
    /// Unknown to this crate version; round-trips losslessly.
    Unknown(UnknownFilterStatusOperator),
}
