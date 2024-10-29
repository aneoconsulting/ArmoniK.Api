mod array_operator;
mod boolean_operator;
mod date_operator;
mod duration_operator;
mod filter;
mod number_operator;
mod status_operator;
mod string_operator;

pub use array_operator::FilterArrayOperator;
pub use boolean_operator::FilterBooleanOperator;
pub use date_operator::FilterDateOperator;
pub use duration_operator::FilterDurationOperator;
pub use filter::{
    FilterArray, FilterBoolean, FilterDate, FilterDuration, FilterNumber, FilterStatus,
    FilterString,
};
pub use number_operator::FilterNumberOperator;
pub use status_operator::FilterStatusOperator;
pub use string_operator::FilterStringOperator;

macro_rules! impl_filter {
    (Filter[$field:ty, $condition:ty]: $api_or:ty [$api_and:ty[$api_field:ty, $api_condition:ty]]) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct Or {
            pub or: Vec<And>,
        }

        super::super::impl_convert!(
            struct Or = $api_or {
                list or,
            }
        );

        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct And {
            pub and: Vec<Field>,
        }

        super::super::impl_convert!(
            struct And = $api_and {
                list and,
            }
        );

        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct Field {
            pub field: $field,
            pub condition: $condition,
        }

        super::super::impl_convert!(
            struct Field = $api_field {
                field = option field,
                condition = option value_condition,
            }
        );

        crate::utils::impl_vec_wrapper!(Or{or: And});
        crate::utils::impl_vec_wrapper!(And{and: Field});
    };
}

pub(crate) use impl_filter;
