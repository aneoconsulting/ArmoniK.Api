use super::super::FilterString;

use crate::{api::v3, impl_filter};

impl_filter!(
    Filter[super::Field, Condition]:
    v3::applications::Filters[
        v3::applications::FiltersAnd[
            v3::applications::FilterField,
            v3::applications::filter_field::ValueCondition
        ]
    ]
);

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
