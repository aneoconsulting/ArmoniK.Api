use super::{
    super::{FilterArray, FilterBoolean, FilterDate, FilterNumber, FilterString, TaskStatus},
    TaskField,
};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskFilters {
    pub or: Vec<TaskFiltersAnd>,
}

impl From<TaskFilters> for v3::tasks::Filters {
    fn from(value: TaskFilters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::Filters> for TaskFilters {
    fn from(value: v3::tasks::Filters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(TaskFilters : Option<v3::tasks::Filters>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskFiltersAnd {
    pub and: Vec<TaskFilterField>,
}

impl From<TaskFiltersAnd> for v3::tasks::FiltersAnd {
    fn from(value: TaskFiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::tasks::FiltersAnd> for TaskFiltersAnd {
    fn from(value: v3::tasks::FiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}
super::super::impl_convert!(TaskFiltersAnd : Option<v3::tasks::FiltersAnd>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskFilterField {
    pub field: TaskField,
    pub condition: TaskFilterCondition,
}

impl From<TaskFilterField> for v3::tasks::FilterField {
    fn from(value: TaskFilterField) -> Self {
        Self {
            field: Some(value.field.into()),
            value_condition: Some(value.condition.into()),
        }
    }
}

impl From<v3::tasks::FilterField> for TaskFilterField {
    fn from(value: v3::tasks::FilterField) -> Self {
        Self {
            field: value.field.unwrap_or_default().into(),
            condition: match value.value_condition {
                Some(cond) => cond.into(),
                None => TaskFilterCondition::String(Default::default()),
            },
        }
    }
}

super::super::impl_convert!(TaskFilterField : Option<v3::tasks::FilterField>);

pub type FilterStatus = super::super::FilterStatus<TaskStatus>;

impl From<FilterStatus> for v3::tasks::FilterStatus {
    fn from(value: FilterStatus) -> Self {
        Self {
            value: value.value as i32,
            operator: value.operator as i32,
        }
    }
}

impl From<v3::tasks::FilterStatus> for FilterStatus {
    fn from(value: v3::tasks::FilterStatus) -> Self {
        Self {
            value: value.value.into(),
            operator: value.operator.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskFilterCondition {
    String(FilterString),
    Number(FilterNumber),
    Boolean(FilterBoolean),
    Status(FilterStatus),
    Date(FilterDate),
    Array(FilterArray),
}

impl Default for TaskFilterCondition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<TaskFilterCondition> for v3::tasks::filter_field::ValueCondition {
    fn from(value: TaskFilterCondition) -> Self {
        match value {
            TaskFilterCondition::String(cond) => Self::FilterString(cond.into()),
            TaskFilterCondition::Number(cond) => Self::FilterNumber(cond.into()),
            TaskFilterCondition::Boolean(cond) => Self::FilterBoolean(cond.into()),
            TaskFilterCondition::Status(cond) => Self::FilterStatus(cond.into()),
            TaskFilterCondition::Date(cond) => Self::FilterDate(cond.into()),
            TaskFilterCondition::Array(cond) => Self::FilterArray(cond.into()),
        }
    }
}

impl From<v3::tasks::filter_field::ValueCondition> for TaskFilterCondition {
    fn from(value: v3::tasks::filter_field::ValueCondition) -> Self {
        match value {
            v3::tasks::filter_field::ValueCondition::FilterString(cond) => {
                Self::String(cond.into())
            }
            v3::tasks::filter_field::ValueCondition::FilterNumber(cond) => {
                Self::Number(cond.into())
            }
            v3::tasks::filter_field::ValueCondition::FilterBoolean(cond) => {
                Self::Boolean(cond.into())
            }
            v3::tasks::filter_field::ValueCondition::FilterStatus(cond) => {
                Self::Status(cond.into())
            }
            v3::tasks::filter_field::ValueCondition::FilterDate(cond) => Self::Date(cond.into()),
            v3::tasks::filter_field::ValueCondition::FilterArray(cond) => Self::Array(cond.into()),
        }
    }
}

super::super::impl_convert!(TaskFilterCondition : Option<v3::tasks::filter_field::ValueCondition>);
