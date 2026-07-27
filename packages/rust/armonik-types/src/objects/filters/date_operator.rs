#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterDateOperator")]
pub enum FilterDateOperator {
    /// Is equal to the specified date.
    #[default]
    Equal,
    /// Is not equal to the specified date.
    NotEqual,
    /// Is before the specified date.
    Before,
    /// Is before or equal to the specified date.
    BeforeOrEqual,
    /// Is After or equal to the specified date.
    AfterOrEqual,
    /// Is after the specified date.
    After,
    /// Unknown to this crate version; round-trips losslessly.
    Other(OtherFilterDateOperator),
}
