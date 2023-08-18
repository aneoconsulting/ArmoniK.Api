use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum FilterNumberOperator {
    #[default]
    Equal = 0,
    NotEqual = 1,
    LessThan = 2,
    LessThanOrEqual = 3,
    GreaterThanOrEqual = 4,
    GreaterThan = 5,
}

impl From<i32> for FilterNumberOperator {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Equal,
            1 => Self::NotEqual,
            2 => Self::LessThan,
            3 => Self::LessThanOrEqual,
            4 => Self::GreaterThanOrEqual,
            5 => Self::GreaterThan,
            _ => Default::default(),
        }
    }
}

impl From<FilterNumberOperator> for v3::FilterNumberOperator {
    fn from(value: FilterNumberOperator) -> Self {
        match value {
            FilterNumberOperator::Equal => Self::Equal,
            FilterNumberOperator::NotEqual => Self::NotEqual,
            FilterNumberOperator::LessThan => Self::LessThan,
            FilterNumberOperator::LessThanOrEqual => Self::LessThanOrEqual,
            FilterNumberOperator::GreaterThanOrEqual => Self::GreaterThanOrEqual,
            FilterNumberOperator::GreaterThan => Self::GreaterThan,
        }
    }
}

impl From<v3::FilterNumberOperator> for FilterNumberOperator {
    fn from(value: v3::FilterNumberOperator) -> Self {
        match value {
            v3::FilterNumberOperator::Equal => Self::Equal,
            v3::FilterNumberOperator::NotEqual => Self::NotEqual,
            v3::FilterNumberOperator::LessThan => Self::LessThan,
            v3::FilterNumberOperator::LessThanOrEqual => Self::LessThanOrEqual,
            v3::FilterNumberOperator::GreaterThanOrEqual => Self::GreaterThanOrEqual,
            v3::FilterNumberOperator::GreaterThan => Self::GreaterThan,
        }
    }
}

super::super::impl_convert!(req FilterNumberOperator : v3::FilterNumberOperator);
