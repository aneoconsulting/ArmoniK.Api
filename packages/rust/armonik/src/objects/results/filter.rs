use super::{
    super::{FilterArray, FilterDate, FilterString, ResultStatus},
    ResultField,
};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultFilters {
    pub or: Vec<ResultFiltersAnd>,
}

impl From<ResultFilters> for v3::results::Filters {
    fn from(value: ResultFilters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::results::Filters> for ResultFilters {
    fn from(value: v3::results::Filters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(ResultFilters : Option<v3::results::Filters>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultFiltersAnd {
    pub and: Vec<ResultFilterField>,
}

impl From<ResultFiltersAnd> for v3::results::FiltersAnd {
    fn from(value: ResultFiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::results::FiltersAnd> for ResultFiltersAnd {
    fn from(value: v3::results::FiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}
super::super::impl_convert!(ResultFiltersAnd : Option<v3::results::FiltersAnd>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultFilterField {
    pub field: ResultField,
    pub condition: ResultFilterCondition,
}

impl From<ResultFilterField> for v3::results::FilterField {
    fn from(value: ResultFilterField) -> Self {
        Self {
            field: Some(value.field.into()),
            value_condition: Some(value.condition.into()),
        }
    }
}

impl From<v3::results::FilterField> for ResultFilterField {
    fn from(value: v3::results::FilterField) -> Self {
        Self {
            field: value.field.unwrap_or_default().into(),
            condition: match value.value_condition {
                Some(cond) => cond.into(),
                None => ResultFilterCondition::String(Default::default()),
            },
        }
    }
}

super::super::impl_convert!(ResultFilterField : Option<v3::results::FilterField>);

pub type FilterStatus = super::super::FilterStatus<ResultStatus>;

impl From<FilterStatus> for v3::results::FilterStatus {
    fn from(value: FilterStatus) -> Self {
        Self {
            value: value.value as i32,
            operator: value.operator as i32,
        }
    }
}

impl From<v3::results::FilterStatus> for FilterStatus {
    fn from(value: v3::results::FilterStatus) -> Self {
        Self {
            value: value.value.into(),
            operator: value.operator.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultFilterCondition {
    String(FilterString),
    Date(FilterDate),
    Array(FilterArray),
    Status(FilterStatus),
}

impl Default for ResultFilterCondition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<ResultFilterCondition> for v3::results::filter_field::ValueCondition {
    fn from(value: ResultFilterCondition) -> Self {
        match value {
            ResultFilterCondition::String(cond) => Self::FilterString(cond.into()),
            ResultFilterCondition::Date(cond) => Self::FilterDate(cond.into()),
            ResultFilterCondition::Array(cond) => Self::FilterArray(cond.into()),
            ResultFilterCondition::Status(cond) => Self::FilterStatus(cond.into()),
        }
    }
}

impl From<v3::results::filter_field::ValueCondition> for ResultFilterCondition {
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
        }
    }
}

super::super::impl_convert!(ResultFilterCondition : Option<v3::results::filter_field::ValueCondition>);
