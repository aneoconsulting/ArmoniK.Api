use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum FilterArrayOperator {
    #[default]
    Contains = 0,
    NotContains = 1,
}

impl From<i32> for FilterArrayOperator {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Contains,
            1 => Self::NotContains,
            _ => Default::default(),
        }
    }
}

impl From<FilterArrayOperator> for v3::FilterArrayOperator {
    fn from(value: FilterArrayOperator) -> Self {
        match value {
            FilterArrayOperator::Contains => Self::Contains,
            FilterArrayOperator::NotContains => Self::NotContains,
        }
    }
}

impl From<v3::FilterArrayOperator> for FilterArrayOperator {
    fn from(value: v3::FilterArrayOperator) -> Self {
        match value {
            v3::FilterArrayOperator::Contains => Self::Contains,
            v3::FilterArrayOperator::NotContains => Self::NotContains,
        }
    }
}

super::super::impl_convert!(req FilterArrayOperator : v3::FilterArrayOperator);
