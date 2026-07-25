//! ArmoniK objects related to common filters

mod array_operator;
mod boolean_operator;
mod date_operator;
mod duration_operator;
mod filter;
mod number_operator;
mod status_operator;
mod string_operator;

pub use array_operator::{FilterArrayOperator, OtherFilterArrayOperator};
pub use boolean_operator::{FilterBooleanOperator, OtherFilterBooleanOperator};
pub use date_operator::{FilterDateOperator, OtherFilterDateOperator};
pub use duration_operator::{FilterDurationOperator, OtherFilterDurationOperator};
pub use filter::{
    FilterArray, FilterBoolean, FilterDate, FilterDuration, FilterNumber, FilterStatus,
    FilterString,
};
pub use number_operator::{FilterNumberOperator, OtherFilterNumberOperator};
pub use status_operator::{FilterStatusOperator, OtherFilterStatusOperator};
pub use string_operator::{FilterStringOperator, OtherFilterStringOperator};

macro_rules! impl_filter {
    // Migrated form: direct wire implementations from the descriptor.
    (Filter[$field:ty, $condition:ty]: protos[$or_proto:literal, $and_proto:literal, $field_proto:literal]) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[armonik(message = $or_proto)]
        pub struct Or {
            pub or: Vec<And>,
        }

        #[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[armonik(message = $and_proto)]
        pub struct And {
            pub and: Vec<Field>,
        }

        #[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[armonik(message = $field_proto)]
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
