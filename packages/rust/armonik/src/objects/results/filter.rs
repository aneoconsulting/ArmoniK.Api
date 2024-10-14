use super::super::{FilterArray, FilterDate, FilterNumber, FilterString, ResultStatus};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Or {
    pub or: Vec<And>,
}

super::super::impl_convert!(
    struct Or = v3::results::Filters {
        list or,
    }
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct And {
    pub and: Vec<Field>,
}

super::super::impl_convert!(
    struct And = v3::results::FiltersAnd {
        list and,
    }
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Field {
    pub field: super::Field,
    pub condition: Condition,
}

super::super::impl_convert!(
    struct Field = v3::results::FilterField {
        field = option field,
        condition = option value_condition,
    }
);

pub type Status = super::super::FilterStatus<ResultStatus>;

super::super::impl_convert!(
    struct Status = v3::results::FilterStatus {
        value = enum value,
        operator = enum operator,
    }
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    String(FilterString),
    Date(FilterDate),
    Array(FilterArray),
    Status(Status),
    Number(FilterNumber),
}

impl Default for Condition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<Condition> for v3::results::filter_field::ValueCondition {
    fn from(value: Condition) -> Self {
        match value {
            Condition::String(cond) => Self::FilterString(cond.into()),
            Condition::Date(cond) => Self::FilterDate(cond.into()),
            Condition::Array(cond) => Self::FilterArray(cond.into()),
            Condition::Status(cond) => Self::FilterStatus(cond.into()),
            Condition::Number(cond) => Self::FilterNumber(cond.into()),
        }
    }
}

impl From<v3::results::filter_field::ValueCondition> for Condition {
    fn from(value: v3::results::filter_field::ValueCondition) -> Self {
        match value {
            v3::results::filter_field::ValueCondition::FilterString(cond) => {
                Self::String(cond.into())
            }
            v3::results::filter_field::ValueCondition::FilterDate(cond) => Self::Date(cond.into()),
            v3::results::filter_field::ValueCondition::FilterArray(cond) => {
                Self::Array(cond.into())
            }
            v3::results::filter_field::ValueCondition::FilterStatus(cond) => {
                Self::Status(cond.into())
            }
            v3::results::filter_field::ValueCondition::FilterNumber(cond) => {
                Self::Number(cond.into())
            }
        }
    }
}

super::super::impl_convert!(req Condition : v3::results::filter_field::ValueCondition);
