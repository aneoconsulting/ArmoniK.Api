use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum FilterDateOperator {
    #[default]
    Equal = 0,
    NotEqual = 1,
    Before = 2,
    BeforeOrEqual = 3,
    AfterOrEqual = 4,
    After = 5,
}

impl From<i32> for FilterDateOperator {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Equal,
            1 => Self::NotEqual,
            2 => Self::Before,
            3 => Self::BeforeOrEqual,
            4 => Self::AfterOrEqual,
            5 => Self::After,
            _ => Default::default(),
        }
    }
}

impl From<FilterDateOperator> for v3::FilterDateOperator {
    fn from(value: FilterDateOperator) -> Self {
        match value {
            FilterDateOperator::Equal => Self::Equal,
            FilterDateOperator::NotEqual => Self::NotEqual,
            FilterDateOperator::Before => Self::Before,
            FilterDateOperator::BeforeOrEqual => Self::BeforeOrEqual,
            FilterDateOperator::AfterOrEqual => Self::AfterOrEqual,
            FilterDateOperator::After => Self::After,
        }
    }
}

impl From<v3::FilterDateOperator> for FilterDateOperator {
    fn from(value: v3::FilterDateOperator) -> Self {
        match value {
            v3::FilterDateOperator::Equal => Self::Equal,
            v3::FilterDateOperator::NotEqual => Self::NotEqual,
            v3::FilterDateOperator::Before => Self::Before,
            v3::FilterDateOperator::BeforeOrEqual => Self::BeforeOrEqual,
            v3::FilterDateOperator::AfterOrEqual => Self::AfterOrEqual,
            v3::FilterDateOperator::After => Self::After,
        }
    }
}

super::super::impl_convert!(req FilterDateOperator : v3::FilterDateOperator);
