#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(enum = "armonik.api.grpc.v1.FilterBooleanOperator")]
pub enum FilterBooleanOperator {
    /// Is the same as the specified boolean.
    #[default]
    Is,
    /// Unknown to this crate version; round-trips losslessly.
    Unknown(UnknownFilterBooleanOperator),
}
