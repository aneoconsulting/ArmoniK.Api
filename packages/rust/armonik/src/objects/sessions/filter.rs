use super::super::{
    FilterArray, FilterBoolean, FilterDate, FilterNumber, FilterString, SessionStatus,
};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Or {
    pub or: Vec<And>,
}

impl From<Or> for v3::sessions::Filters {
    fn from(value: Or) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::sessions::Filters> for Or {
    fn from(value: v3::sessions::Filters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(Or : Option<v3::sessions::Filters>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct And {
    pub and: Vec<Field>,
}

impl From<And> for v3::sessions::FiltersAnd {
    fn from(value: And) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::sessions::FiltersAnd> for And {
    fn from(value: v3::sessions::FiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}
super::super::impl_convert!(And : Option<v3::sessions::FiltersAnd>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Field {
    pub field: super::Field,
    pub condition: Condition,
}

impl From<Field> for v3::sessions::FilterField {
    fn from(value: Field) -> Self {
        Self {
            field: Some(value.field.into()),
            value_condition: Some(value.condition.into()),
        }
    }
}

impl From<v3::sessions::FilterField> for Field {
    fn from(value: v3::sessions::FilterField) -> Self {
        Self {
            field: value.field.unwrap_or_default().into(),
            condition: match value.value_condition {
                Some(cond) => cond.into(),
                None => Condition::String(Default::default()),
            },
        }
    }
}

super::super::impl_convert!(Field : Option<v3::sessions::FilterField>);

pub type Status = super::super::FilterStatus<SessionStatus>;

impl From<Status> for v3::sessions::FilterStatus {
    fn from(value: Status) -> Self {
        Self {
            value: value.value as i32,
            operator: value.operator as i32,
        }
    }
}

impl From<v3::sessions::FilterStatus> for Status {
    fn from(value: v3::sessions::FilterStatus) -> Self {
        Self {
            value: value.value.into(),
            operator: value.operator.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    String(FilterString),
    Number(FilterNumber),
    Boolean(FilterBoolean),
    Status(Status),
    Date(FilterDate),
    Array(FilterArray),
}

impl Default for Condition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<Condition> for v3::sessions::filter_field::ValueCondition {
    fn from(value: Condition) -> Self {
        match value {
            Condition::String(cond) => Self::FilterString(cond.into()),
            Condition::Number(cond) => Self::FilterNumber(cond.into()),
            Condition::Boolean(cond) => Self::FilterBoolean(cond.into()),
            Condition::Status(cond) => Self::FilterStatus(cond.into()),
            Condition::Date(cond) => Self::FilterDate(cond.into()),
            Condition::Array(cond) => Self::FilterArray(cond.into()),
        }
    }
}

impl From<v3::sessions::filter_field::ValueCondition> for Condition {
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

super::super::impl_convert!(Condition : Option<v3::sessions::filter_field::ValueCondition>);
