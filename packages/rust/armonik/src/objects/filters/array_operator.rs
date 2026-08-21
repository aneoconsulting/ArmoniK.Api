#[armonik_macros::enumeration("armonik.api.grpc.v1.FilterArrayOperator")]
#[derive(Debug, Clone, Copy, Default)]
pub enum FilterArrayOperator {
    /// Contains the specified element.
    #[default]
    Contains,
    /// Does not contain the specified element.
    NotContains,
    /// Unknown to this crate version; round-trips losslessly.
    Unknown(UnknownFilterArrayOperator),
}
