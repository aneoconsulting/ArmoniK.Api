use super::super::{FilterArray, FilterBoolean, FilterNumber, FilterString};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Or {
    pub or: Vec<And>,
}

impl From<Or> for v3::partitions::Filters {
    fn from(value: Or) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::partitions::Filters> for Or {
    fn from(value: v3::partitions::Filters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(Or : Option<v3::partitions::Filters>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct And {
    pub and: Vec<Field>,
}

impl From<And> for v3::partitions::FiltersAnd {
    fn from(value: And) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::partitions::FiltersAnd> for And {
    fn from(value: v3::partitions::FiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}
super::super::impl_convert!(And : Option<v3::partitions::FiltersAnd>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Field {
    pub field: super::Field,
    pub condition: Condition,
}

impl From<Field> for v3::partitions::FilterField {
    fn from(value: Field) -> Self {
        Self {
            field: Some(value.field.into()),
            value_condition: Some(value.condition.into()),
        }
    }
}

impl From<v3::partitions::FilterField> for Field {
    fn from(value: v3::partitions::FilterField) -> Self {
        Self {
            field: value.field.unwrap_or_default().into(),
            condition: match value.value_condition {
                Some(cond) => cond.into(),
                None => Condition::String(Default::default()),
            },
        }
    }
}

super::super::impl_convert!(Field : Option<v3::partitions::FilterField>);

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

super::super::impl_convert!(Condition : Option<v3::partitions::filter_field::ValueCondition>);
