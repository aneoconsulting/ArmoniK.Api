use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum FilterStatusOperator {
    #[default]
    Equal = 0,
    NotEqual = 1,
}

impl From<i32> for FilterStatusOperator {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Equal,
            1 => Self::NotEqual,
            _ => Default::default(),
        }
    }
}

impl From<FilterStatusOperator> for v3::FilterStatusOperator {
    fn from(value: FilterStatusOperator) -> Self {
        match value {
            FilterStatusOperator::Equal => Self::Equal,
            FilterStatusOperator::NotEqual => Self::NotEqual,
        }
    }
}

impl From<v3::FilterStatusOperator> for FilterStatusOperator {
    fn from(value: v3::FilterStatusOperator) -> Self {
        match value {
            v3::FilterStatusOperator::Equal => Self::Equal,
            v3::FilterStatusOperator::NotEqual => Self::NotEqual,
        }
    }
}

super::super::impl_convert!(req FilterStatusOperator : v3::FilterStatusOperator);
