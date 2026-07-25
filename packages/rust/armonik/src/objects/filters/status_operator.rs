#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterStatusOperator")]
pub enum FilterStatusOperator {
    /// Is equal to the specified status.
    #[default]
    Equal,
    /// Is not equal to the specified status.
    NotEqual,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterStatusOperator),
}
