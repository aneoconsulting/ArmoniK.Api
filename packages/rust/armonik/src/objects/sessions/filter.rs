use super::{
    super::{FilterArray, FilterBoolean, FilterDate, FilterNumber, FilterString, SessionStatus},
    SessionField,
};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionFilters {
    pub or: Vec<SessionFiltersAnd>,
}

impl From<SessionFilters> for v3::sessions::Filters {
    fn from(value: SessionFilters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::sessions::Filters> for SessionFilters {
    fn from(value: v3::sessions::Filters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(SessionFilters : Option<v3::sessions::Filters>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionFiltersAnd {
    pub and: Vec<SessionFilterField>,
}

impl From<SessionFiltersAnd> for v3::sessions::FiltersAnd {
    fn from(value: SessionFiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::sessions::FiltersAnd> for SessionFiltersAnd {
    fn from(value: v3::sessions::FiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}
super::super::impl_convert!(SessionFiltersAnd : Option<v3::sessions::FiltersAnd>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionFilterField {
    pub field: SessionField,
    pub condition: SessionFilterCondition,
}

impl From<SessionFilterField> for v3::sessions::FilterField {
    fn from(value: SessionFilterField) -> Self {
        Self {
            field: Some(value.field.into()),
            value_condition: Some(value.condition.into()),
        }
    }
}

impl From<v3::sessions::FilterField> for SessionFilterField {
    fn from(value: v3::sessions::FilterField) -> Self {
        Self {
            field: value.field.unwrap_or_default().into(),
            condition: match value.value_condition {
                Some(cond) => cond.into(),
                None => SessionFilterCondition::String(Default::default()),
            },
        }
    }
}

super::super::impl_convert!(SessionFilterField : Option<v3::sessions::FilterField>);

pub type FilterStatus = super::super::FilterStatus<SessionStatus>;

impl From<FilterStatus> for v3::sessions::FilterStatus {
    fn from(value: FilterStatus) -> Self {
        Self {
            value: value.value as i32,
            operator: value.operator as i32,
        }
    }
}

impl From<v3::sessions::FilterStatus> for FilterStatus {
    fn from(value: v3::sessions::FilterStatus) -> Self {
        Self {
            value: value.value.into(),
            operator: value.operator.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionFilterCondition {
    String(FilterString),
    Number(FilterNumber),
    Boolean(FilterBoolean),
    Status(FilterStatus),
    Date(FilterDate),
    Array(FilterArray),
}

impl Default for SessionFilterCondition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<SessionFilterCondition> for v3::sessions::filter_field::ValueCondition {
    fn from(value: SessionFilterCondition) -> Self {
        match value {
            SessionFilterCondition::String(cond) => Self::FilterString(cond.into()),
            SessionFilterCondition::Number(cond) => Self::FilterNumber(cond.into()),
            SessionFilterCondition::Boolean(cond) => Self::FilterBoolean(cond.into()),
            SessionFilterCondition::Status(cond) => Self::FilterStatus(cond.into()),
            SessionFilterCondition::Date(cond) => Self::FilterDate(cond.into()),
            SessionFilterCondition::Array(cond) => Self::FilterArray(cond.into()),
        }
    }
}

impl From<v3::sessions::filter_field::ValueCondition> for SessionFilterCondition {
    fn from(value: v3::sessions::filter_field::ValueCondition) -> Self {
        match value {
            v3::sessions::filter_field::ValueCondition::FilterString(cond) => {
                Self::String(cond.into())
            }
            v3::sessions::filter_field::ValueCondition::FilterNumber(cond) => {
                Self::Number(cond.into())
            }
            v3::sessions::filter_field::ValueCondition::FilterBoolean(cond) => {
                Self::Boolean(cond.into())
            }
            v3::sessions::filter_field::ValueCondition::FilterStatus(cond) => {
                Self::Status(cond.into())
            }
            v3::sessions::filter_field::ValueCondition::FilterDate(cond) => Self::Date(cond.into()),
            v3::sessions::filter_field::ValueCondition::FilterArray(cond) => {
                Self::Array(cond.into())
            }
        }
    }
}

super::super::impl_convert!(SessionFilterCondition : Option<v3::sessions::filter_field::ValueCondition>);
