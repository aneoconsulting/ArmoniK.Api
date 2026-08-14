#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, Default)]
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
    Unknown(UnknownFilterStringOperator),
}
