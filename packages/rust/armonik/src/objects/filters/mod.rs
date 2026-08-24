//! ArmoniK objects related to common filters

#[doc(hidden)]
pub mod array_operator;
#[doc(hidden)]
pub mod boolean_operator;
#[doc(hidden)]
pub mod date_operator;
#[doc(hidden)]
pub mod duration_operator;
#[doc(hidden)]
pub mod filter;
#[doc(hidden)]
pub mod number_operator;
#[doc(hidden)]
pub mod status_operator;
#[doc(hidden)]
pub mod string_operator;

pub use array_operator::{FilterArrayOperator, UnknownFilterArrayOperator};
pub use boolean_operator::{FilterBooleanOperator, UnknownFilterBooleanOperator};
pub use date_operator::{FilterDateOperator, UnknownFilterDateOperator};
pub use duration_operator::{FilterDurationOperator, UnknownFilterDurationOperator};
pub use filter::{
    FilterArray, FilterBoolean, FilterDate, FilterDuration, FilterNumber, FilterStatus,
    FilterString,
};
pub use number_operator::{FilterNumberOperator, UnknownFilterNumberOperator};
pub use status_operator::{FilterStatusOperator, UnknownFilterStatusOperator};
pub use string_operator::{FilterStringOperator, UnknownFilterStringOperator};

/// The three types one filter family is made of, against the three proto messages it declares:
/// `Or` over `And` over `Field`, which pairs the field to filter on with the condition to apply.
/// The field enum and the condition are the family's own; everything else is the same shape for
/// every service.
macro_rules! impl_filter {
    (Filter[$field:ty, $condition:ty]: protos[$or_proto:literal, $and_proto:literal, $field_proto:literal]) => {
        #[armonik_macros::message($or_proto)]
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct Or {
            pub or: Vec<And>,
        }

        #[armonik_macros::message($and_proto)]
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct And {
            pub and: Vec<Field>,
        }

        #[armonik_macros::message($field_proto)]
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct Field {
            pub field: $field,
            #[armonik(rename = "value_condition")]
            pub condition: $condition,
        }

        crate::utils::impl_vec_wrapper!(Or { or: And });
        crate::utils::impl_vec_wrapper!(And { and: Field });
    };
}

pub(crate) use impl_filter;
