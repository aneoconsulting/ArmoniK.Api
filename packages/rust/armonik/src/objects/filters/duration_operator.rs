#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(enum = "armonik.api.grpc.v1.FilterDurationOperator")]
pub enum FilterDurationOperator {
    /// Is equal to the specified duration.
    #[default]
    Equal,
    /// Is not equal to the specified duration.
    NotEqual,
    /// Is shorter than the specified duration.
    ShorterThan,
    /// Is shorter or equal to the specified duration.
    ShorterThanOrEqual,
    /// Is longer or equal to the specified duration.
    LongerThanOrEqual,
    /// Is longer than the specified duration.
    LongerThan,
    /// Unknown to this crate version; round-trips losslessly.
    Unknown(UnknownFilterDurationOperator),
}
