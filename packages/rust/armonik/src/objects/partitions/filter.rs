use super::super::{FilterArray, FilterBoolean, FilterNumber, FilterString};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Or {
    pub or: Vec<And>,
}

super::super::impl_convert!(
    struct Or = v3::partitions::Filters {
        list or,
    }
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct And {
    pub and: Vec<Field>,
}

super::super::impl_convert!(
    struct And = v3::partitions::FiltersAnd {
        list and,
    }
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Field {
    pub field: super::Field,
    pub condition: Condition,
}

super::super::impl_convert!(
    struct Field = v3::partitions::FilterField {
        field = option field,
        condition = option value_condition,
    }
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    String(FilterString),
    Number(FilterNumber),
    Boolean(FilterBoolean),
    Array(FilterArray),
}

impl Default for Condition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<Condition> for v3::partitions::filter_field::ValueCondition {
    fn from(value: Condition) -> Self {
        match value {
            Condition::String(cond) => Self::FilterString(cond.into()),
            Condition::Number(cond) => Self::FilterNumber(cond.into()),
            Condition::Boolean(cond) => Self::FilterBoolean(cond.into()),
            Condition::Array(cond) => Self::FilterArray(cond.into()),
        }
    }
}

impl From<v3::partitions::filter_field::ValueCondition> for Condition {
    fn from(value: v3::partitions::filter_field::ValueCondition) -> Self {
        match value {
            v3::partitions::filter_field::ValueCondition::FilterString(cond) => {
                Self::String(cond.into())
            }
            v3::partitions::filter_field::ValueCondition::FilterNumber(cond) => {
                Self::Number(cond.into())
            }
            v3::partitions::filter_field::ValueCondition::FilterBoolean(cond) => {
                Self::Boolean(cond.into())
            }
            v3::partitions::filter_field::ValueCondition::FilterArray(cond) => {
                Self::Array(cond.into())
            }
        }
    }
}

super::super::impl_convert!(req Condition : v3::partitions::filter_field::ValueCondition);
