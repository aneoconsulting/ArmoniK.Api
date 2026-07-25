use crate::api::v3;

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

        impl From<$name> for v3::$name {
            fn from(value: $name) -> Self {
                Self {
                    value: value.value,
                    operator: v3::$op::from(value.operator) as i32,
                }
            }
        }

        impl From<v3::$name> for $name {
            fn from(value: v3::$name) -> Self {
                Self {
                    value: value.value,
                    operator: value.operator.into(),
                }
            }
        }

        super::super::impl_convert!(req $name : v3::$name);
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

impl From<FilterDate> for v3::FilterDate {
    fn from(value: FilterDate) -> Self {
        Self {
            value: Some(value.value),
            operator: v3::FilterDateOperator::from(value.operator) as i32,
        }
    }
}

impl From<v3::FilterDate> for FilterDate {
    fn from(value: v3::FilterDate) -> Self {
        Self {
            value: value.value.unwrap_or_default(),
            operator: value.operator.into(),
        }
    }
}

super::super::impl_convert!(req FilterDate : v3::FilterDate);

#[derive(Debug, Clone, Default, PartialEq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.FilterDuration")]
pub struct FilterDuration {
    #[cfg_attr(feature = "serde", serde(with = "crate::utils::serde_duration"))]
    pub value: prost_types::Duration,
    pub operator: FilterDurationOperator,
}

impl From<FilterDuration> for v3::FilterDuration {
    fn from(value: FilterDuration) -> Self {
        Self {
            value: Some(value.value),
            operator: v3::FilterDurationOperator::from(value.operator) as i32,
        }
    }
}

impl From<v3::FilterDuration> for FilterDuration {
    fn from(value: v3::FilterDuration) -> Self {
        Self {
            value: value.value.unwrap_or_default(),
            operator: value.operator.into(),
        }
    }
}

impl Eq for FilterDuration {}

super::super::impl_convert!(req FilterDuration : v3::FilterDuration);

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
