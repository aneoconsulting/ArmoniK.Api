use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(i32)]
pub enum FilterDurationOperator {
    /// Is equal to the specified duration.
    #[default]
    Equal = 0,
    /// Is not equal to the specified duration.
    NotEqual = 1,
    /// Is shorter than the specified duration.
    ShorterThan = 2,
    /// Is shorter or equal to the specified duration.
    ShorterThanOrEqual = 3,
    /// Is longer or equal to the specified duration.
    LongerThanOrEqual = 4,
    /// Is longer than the specified duration.
    LongerThan = 5,
}

impl From<i32> for FilterDurationOperator {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Equal,
            1 => Self::NotEqual,
            2 => Self::ShorterThan,
            3 => Self::ShorterThanOrEqual,
            4 => Self::LongerThanOrEqual,
            5 => Self::LongerThan,
            _ => Default::default(),
        }
    }
}

impl From<FilterDurationOperator> for v3::FilterDurationOperator {
    fn from(value: FilterDurationOperator) -> Self {
        match value {
            FilterDurationOperator::Equal => Self::Equal,
            FilterDurationOperator::NotEqual => Self::NotEqual,
            FilterDurationOperator::ShorterThan => Self::ShorterThan,
            FilterDurationOperator::ShorterThanOrEqual => Self::ShorterThanOrEqual,
            FilterDurationOperator::LongerThanOrEqual => Self::LongerThanOrEqual,
            FilterDurationOperator::LongerThan => Self::LongerThan,
        }
    }
}

impl From<v3::FilterDurationOperator> for FilterDurationOperator {
    fn from(value: v3::FilterDurationOperator) -> Self {
        match value {
            v3::FilterDurationOperator::Equal => Self::Equal,
            v3::FilterDurationOperator::NotEqual => Self::NotEqual,
            v3::FilterDurationOperator::ShorterThan => Self::ShorterThan,
            v3::FilterDurationOperator::ShorterThanOrEqual => Self::ShorterThanOrEqual,
            v3::FilterDurationOperator::LongerThanOrEqual => Self::LongerThanOrEqual,
            v3::FilterDurationOperator::LongerThan => Self::LongerThan,
        }
    }
}

super::super::impl_convert!(req FilterDurationOperator : v3::FilterDurationOperator);
