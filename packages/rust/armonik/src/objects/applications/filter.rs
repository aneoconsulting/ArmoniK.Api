use super::{super::FilterString, ApplicationField};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationFilters {
    pub or: Vec<ApplicationFiltersAnd>,
}

impl From<ApplicationFilters> for v3::applications::Filters {
    fn from(value: ApplicationFilters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::applications::Filters> for ApplicationFilters {
    fn from(value: v3::applications::Filters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(ApplicationFilters : Option<v3::applications::Filters>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationFiltersAnd {
    pub and: Vec<ApplicationFilterField>,
}

impl From<ApplicationFiltersAnd> for v3::applications::FiltersAnd {
    fn from(value: ApplicationFiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::applications::FiltersAnd> for ApplicationFiltersAnd {
    fn from(value: v3::applications::FiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}
super::super::impl_convert!(ApplicationFiltersAnd : Option<v3::applications::FiltersAnd>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplicationFilterField {
    pub field: ApplicationField,
    pub condition: ApplicationFilterCondition,
}

impl From<ApplicationFilterField> for v3::applications::FilterField {
    fn from(value: ApplicationFilterField) -> Self {
        Self {
            field: Some(value.field.into()),
            value_condition: Some(value.condition.into()),
        }
    }
}

impl From<v3::applications::FilterField> for ApplicationFilterField {
    fn from(value: v3::applications::FilterField) -> Self {
        Self {
            field: value.field.unwrap_or_default().into(),
            condition: match value.value_condition {
                Some(cond) => cond.into(),
                None => ApplicationFilterCondition::String(Default::default()),
            },
        }
    }
}

super::super::impl_convert!(ApplicationFilterField : Option<v3::applications::FilterField>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationFilterCondition {
    String(FilterString),
}

impl Default for ApplicationFilterCondition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<ApplicationFilterCondition> for v3::applications::filter_field::ValueCondition {
    fn from(value: ApplicationFilterCondition) -> Self {
        match value {
            ApplicationFilterCondition::String(cond) => Self::FilterString(cond.into()),
        }
    }
}

impl From<v3::applications::filter_field::ValueCondition> for ApplicationFilterCondition {
    fn from(value: v3::applications::filter_field::ValueCondition) -> Self {
        match value {
            v3::applications::filter_field::ValueCondition::FilterString(cond) => {
                Self::String(cond.into())
            }
        }
    }
}

super::super::impl_convert!(ApplicationFilterCondition : Option<v3::applications::filter_field::ValueCondition>);
