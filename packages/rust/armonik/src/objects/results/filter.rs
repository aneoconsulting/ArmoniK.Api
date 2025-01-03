use super::super::{FilterArray, FilterDate, FilterNumber, FilterString, ResultStatus};

use crate::{api::v3, impl_filter};

impl_filter!(
    Filter[super::Field, Condition]:
    v3::results::Filters[
        v3::results::FiltersAnd[
            v3::results::FilterField,
            v3::results::filter_field::ValueCondition
        ]
    ]
);

pub type Status = super::super::FilterStatus<ResultStatus>;

super::super::impl_convert!(
    struct Status = v3::results::FilterStatus {
        value = enum value,
        operator = enum operator,
    }
);

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
