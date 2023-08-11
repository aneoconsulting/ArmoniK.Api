use crate::api::v3;

use super::{
    FilterArrayOperator, FilterBooleanOperator, FilterDateOperator, FilterNumberOperator,
    FilterStringOperator,
};

macro_rules! impl_filter {
    ($name:ident => $type:ty : $op:ident) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

        super::super::impl_convert!($name : Option<v3::$name>);
    };
}

impl_filter!(FilterString => String: FilterStringOperator);
impl_filter!(FilterNumber => i64: FilterNumberOperator);
impl_filter!(FilterArray => String: FilterArrayOperator);
impl_filter!(FilterBoolean => bool: FilterBooleanOperator);

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct FilterDate {
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

super::super::impl_convert!(FilterDate : Option<v3::FilterDate>);
