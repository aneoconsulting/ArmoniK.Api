use super::super::{
    FilterArray, FilterBoolean, FilterDate, FilterDuration, FilterNumber, FilterString,
    SessionStatus,
};

use crate::{api::v3, impl_filter};

impl_filter!(
    Filter[super::Field, Condition]:
    v3::sessions::Filters[
        v3::sessions::FiltersAnd[
            v3::sessions::FilterField,
            v3::sessions::filter_field::ValueCondition
        ]
    ]
);

pub type Status = super::super::FilterStatus<SessionStatus>;

super::super::impl_convert!(
    struct Status = v3::sessions::FilterStatus {
        value = enum value,
        operator = enum operator,
    }
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    String(FilterString),
    Number(FilterNumber),
    Boolean(FilterBoolean),
    Status(Status),
    Date(FilterDate),
    Duration(FilterDuration),
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
            Condition::Duration(cond) => Self::FilterDuration(cond.into()),
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
            v3::sessions::filter_field::ValueCondition::FilterDuration(cond) => {
                Self::Duration(cond.into())
            }
        }
    }
}

super::super::impl_convert!(req Condition : v3::sessions::filter_field::ValueCondition);
