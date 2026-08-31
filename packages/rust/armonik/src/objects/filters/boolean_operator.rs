#[armonik_macros::enumeration("armonik.api.grpc.v1.FilterBooleanOperator")]
#[derive(Debug, Clone, Copy, Default)]
pub enum FilterBooleanOperator {
    /// Is the same as the specified boolean.
    #[default]
    Is,
    /// Unknown to this crate version; round-trips losslessly.
    Unknown(UnknownFilterBooleanOperator),
}
