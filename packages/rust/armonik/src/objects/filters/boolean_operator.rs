use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum FilterBooleanOperator {
    #[default]
    Is = 0,
}

impl From<i32> for FilterBooleanOperator {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Is,
            _ => Default::default(),
        }
    }
}

impl From<FilterBooleanOperator> for v3::FilterBooleanOperator {
    fn from(value: FilterBooleanOperator) -> Self {
        match value {
            FilterBooleanOperator::Is => Self::Is,
        }
    }
}

impl From<v3::FilterBooleanOperator> for FilterBooleanOperator {
    fn from(value: v3::FilterBooleanOperator) -> Self {
        match value {
            v3::FilterBooleanOperator::Is => Self::Is,
        }
    }
}

super::super::impl_convert!(req FilterBooleanOperator : v3::FilterBooleanOperator);
