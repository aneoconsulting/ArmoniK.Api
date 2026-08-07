#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterArrayOperator")]
pub enum FilterArrayOperator {
    /// Contains the specified element.
    #[default]
    Contains,
    /// Does not contain the specified element.
    NotContains,
    /// Unknown to this crate version; round-trips losslessly.
    Unknown(UnknownFilterArrayOperator),
}
