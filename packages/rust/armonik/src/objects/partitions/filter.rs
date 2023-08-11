use super::{
    super::{FilterArray, FilterBoolean, FilterNumber, FilterString},
    PartitionField,
};

use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartitionFilters {
    pub or: Vec<PartitionFiltersAnd>,
}

impl From<PartitionFilters> for v3::partitions::Filters {
    fn from(value: PartitionFilters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::partitions::Filters> for PartitionFilters {
    fn from(value: v3::partitions::Filters) -> Self {
        Self {
            or: value.or.into_iter().map(Into::into).collect(),
        }
    }
}

super::super::impl_convert!(PartitionFilters : Option<v3::partitions::Filters>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartitionFiltersAnd {
    pub and: Vec<PartitionFilterField>,
}

impl From<PartitionFiltersAnd> for v3::partitions::FiltersAnd {
    fn from(value: PartitionFiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<v3::partitions::FiltersAnd> for PartitionFiltersAnd {
    fn from(value: v3::partitions::FiltersAnd) -> Self {
        Self {
            and: value.and.into_iter().map(Into::into).collect(),
        }
    }
}
super::super::impl_convert!(PartitionFiltersAnd : Option<v3::partitions::FiltersAnd>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartitionFilterField {
    pub field: PartitionField,
    pub condition: PartitionFilterCondition,
}

impl From<PartitionFilterField> for v3::partitions::FilterField {
    fn from(value: PartitionFilterField) -> Self {
        Self {
            field: Some(value.field.into()),
            value_condition: Some(value.condition.into()),
        }
    }
}

impl From<v3::partitions::FilterField> for PartitionFilterField {
    fn from(value: v3::partitions::FilterField) -> Self {
        Self {
            field: value.field.unwrap_or_default().into(),
            condition: match value.value_condition {
                Some(cond) => cond.into(),
                None => PartitionFilterCondition::String(Default::default()),
            },
        }
    }
}

super::super::impl_convert!(PartitionFilterField : Option<v3::partitions::FilterField>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionFilterCondition {
    String(FilterString),
    Number(FilterNumber),
    Boolean(FilterBoolean),
    Array(FilterArray),
}

impl Default for PartitionFilterCondition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}

impl From<PartitionFilterCondition> for v3::partitions::filter_field::ValueCondition {
    fn from(value: PartitionFilterCondition) -> Self {
        match value {
            PartitionFilterCondition::String(cond) => Self::FilterString(cond.into()),
            PartitionFilterCondition::Number(cond) => Self::FilterNumber(cond.into()),
            PartitionFilterCondition::Boolean(cond) => Self::FilterBoolean(cond.into()),
            PartitionFilterCondition::Array(cond) => Self::FilterArray(cond.into()),
        }
    }
}

impl From<v3::partitions::filter_field::ValueCondition> for PartitionFilterCondition {
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

super::super::impl_convert!(PartitionFilterCondition : Option<v3::partitions::filter_field::ValueCondition>);
