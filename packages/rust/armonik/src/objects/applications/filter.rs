use super::super::FilterString;

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Or {
    pub or: Vec<And>,
}

impl From<Or> for v3::applications::Filters {
    fn from(value: Or) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::applications::Filters> for Or {
    fn from(value: v3::applications::Filters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(Or : Option<v3::applications::Filters>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct And {
    pub and: Vec<Field>,
}

impl From<And> for v3::applications::FiltersAnd {
    fn from(value: And) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::applications::FiltersAnd> for And {
    fn from(value: v3::applications::FiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}
super::super::impl_convert!(And : Option<v3::applications::FiltersAnd>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Field {
    pub field: super::Field,
    pub condition: Condition,
}

impl From<Field> for v3::applications::FilterField {
    fn from(value: Field) -> Self {
        Self {
            field: Some(value.field.into()),
            value_condition: Some(value.condition.into()),
        }
    }
}

impl From<v3::applications::FilterField> for Field {
    fn from(value: v3::applications::FilterField) -> Self {
        Self {
            field: value.field.unwrap_or_default().into(),
            condition: match value.value_condition {
                Some(cond) => cond.into(),
                None => Condition::String(Default::default()),
            },
        }
    }
}

super::super::impl_convert!(Field : Option<v3::applications::FilterField>);

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

super::super::impl_convert!(Condition : Option<v3::applications::filter_field::ValueCondition>);
