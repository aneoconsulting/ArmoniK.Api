use super::super::FilterString;

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Or {
    pub or: Vec<And>,
}

super::super::impl_convert!(struct Or = v3::applications::Filters { list or });

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct And {
    pub and: Vec<Field>,
}

super::super::impl_convert!(struct And = v3::applications::FiltersAnd { list and });

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Field {
    pub field: super::Field,
    pub condition: Condition,
}

super::super::impl_convert!(struct Field = v3::applications::FilterField { field = option field, condition = option value_condition });

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    String(FilterString),
}

impl Default for Condition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<Condition> for v3::applications::filter_field::ValueCondition {
    fn from(value: Condition) -> Self {
        match value {
            Condition::String(cond) => Self::FilterString(cond.into()),
        }
    }
}

impl From<v3::applications::filter_field::ValueCondition> for Condition {
    fn from(value: v3::applications::filter_field::ValueCondition) -> Self {
        match value {
            v3::applications::filter_field::ValueCondition::FilterString(cond) => {
                Self::String(cond.into())
            }
        }
    }
}

super::super::impl_convert!(req Condition : v3::applications::filter_field::ValueCondition);
