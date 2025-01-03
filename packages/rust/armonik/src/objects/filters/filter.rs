use crate::api::v3;

use super::{
    FilterArrayOperator, FilterBooleanOperator, FilterDateOperator, FilterDurationOperator,
    FilterNumberOperator, FilterStatusOperator, FilterStringOperator,
};

macro_rules! impl_filter_condition {
    ($name:ident => $type:ty : $op:ident) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl_filter_condition!(FilterString => String: FilterStringOperator);
impl_filter_condition!(FilterNumber => i64: FilterNumberOperator);
impl_filter_condition!(FilterArray => String: FilterArrayOperator);
impl_filter_condition!(FilterBoolean => bool: FilterBooleanOperator);

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FilterStatus<T> {
    pub value: T,
    pub operator: FilterStatusOperator,
}
