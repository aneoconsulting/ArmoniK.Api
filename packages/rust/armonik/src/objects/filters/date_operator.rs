#[armonik_macros::enumeration("armonik.api.grpc.v1.FilterDateOperator")]
#[derive(Debug, Clone, Copy, Default)]
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
    Unknown(UnknownFilterDateOperator),
}
