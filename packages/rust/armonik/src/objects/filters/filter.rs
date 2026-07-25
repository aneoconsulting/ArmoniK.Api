use super::{
    FilterArrayOperator, FilterBooleanOperator, FilterDateOperator, FilterDurationOperator,
    FilterNumberOperator, FilterStatusOperator, FilterStringOperator,
};

macro_rules! impl_filter_condition {
    ($name:ident, $proto:literal => $type:ty : $op:ident) => {
        #[derive(
            Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message,
        )]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[armonik(message = $proto)]
        pub struct $name {
            pub value: $type,
            pub operator: $op,
        }
    };
}

impl_filter_condition!(FilterString, "armonik.api.grpc.v1.FilterString" => String: FilterStringOperator);
impl_filter_condition!(FilterNumber, "armonik.api.grpc.v1.FilterNumber" => i64: FilterNumberOperator);
impl_filter_condition!(FilterArray, "armonik.api.grpc.v1.FilterArray" => String: FilterArrayOperator);
impl_filter_condition!(FilterBoolean, "armonik.api.grpc.v1.FilterBoolean" => bool: FilterBooleanOperator);

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.FilterDate")]
pub struct FilterDate {
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_timestamp"))]
    pub value: prost_types::Timestamp,
    pub operator: FilterDateOperator,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.FilterDuration")]
pub struct FilterDuration {
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_duration"))]
    pub value: prost_types::Duration,
    pub operator: FilterDurationOperator,
}

/// Stands for the per-service `FilterStatus` messages, whose concrete
/// instantiations are validated by the differential harness.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(generic)]
pub struct FilterStatus<T> {
    #[armonik(tag = 1)]
    pub value: T,
    #[armonik(tag = 2)]
    pub operator: FilterStatusOperator,
}
