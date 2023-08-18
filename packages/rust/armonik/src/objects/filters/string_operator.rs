use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum FilterStringOperator {
    #[default]
    Equal = 0,
    NotEqual = 1,
    Contains = 2,
    NotContains = 3,
    StartsWith = 4,
    EndsWith = 5,
}

impl From<i32> for FilterStringOperator {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Equal,
            1 => Self::NotEqual,
            2 => Self::Contains,
            3 => Self::NotContains,
            4 => Self::StartsWith,
            5 => Self::EndsWith,
            _ => Default::default(),
        }
    }
}

impl From<FilterStringOperator> for v3::FilterStringOperator {
    fn from(value: FilterStringOperator) -> Self {
        match value {
            FilterStringOperator::Equal => Self::Equal,
            FilterStringOperator::NotEqual => Self::NotEqual,
            FilterStringOperator::Contains => Self::Contains,
            FilterStringOperator::NotContains => Self::NotContains,
            FilterStringOperator::StartsWith => Self::StartsWith,
            FilterStringOperator::EndsWith => Self::EndsWith,
        }
    }
}

impl From<v3::FilterStringOperator> for FilterStringOperator {
    fn from(value: v3::FilterStringOperator) -> Self {
        match value {
            v3::FilterStringOperator::Equal => Self::Equal,
            v3::FilterStringOperator::NotEqual => Self::NotEqual,
            v3::FilterStringOperator::Contains => Self::Contains,
            v3::FilterStringOperator::NotContains => Self::NotContains,
            v3::FilterStringOperator::StartsWith => Self::StartsWith,
            v3::FilterStringOperator::EndsWith => Self::EndsWith,
        }
    }
}

super::super::impl_convert!(req FilterStringOperator : v3::FilterStringOperator);
