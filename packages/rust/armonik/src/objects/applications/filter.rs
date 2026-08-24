use super::super::FilterString;

use crate::impl_filter;

impl_filter!(
    Filter[super::Field, Condition]:
    protos[
        "armonik.api.grpc.v1.applications.Filters",
        "armonik.api.grpc.v1.applications.FiltersAnd",
        "armonik.api.grpc.v1.applications.FilterField"
    ]
);

#[armonik_macros::oneof("armonik.api.grpc.v1.applications.FilterField.value_condition")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Condition {
    /// No condition. A `FilterField` that names a field but no condition cannot be evaluated;
    /// reading it as the first condition holding its defaults would filter on something else.
    #[default]
    Invalid,
    #[armonik(rename = "filter_string")]
    String(FilterString),
}
