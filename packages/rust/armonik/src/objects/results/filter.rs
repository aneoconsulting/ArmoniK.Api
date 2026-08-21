use super::super::{FilterArray, FilterDate, FilterNumber, FilterString, ResultStatus};

use crate::impl_filter;

impl_filter!(
    Filter[super::Field, Condition]:
    protos[
        "armonik.api.grpc.v1.results.Filters",
        "armonik.api.grpc.v1.results.FiltersAnd",
        "armonik.api.grpc.v1.results.FilterField"
    ]
);

#[armonik_macros::alias("armonik.api.grpc.v1.results.FilterStatus")]
pub type Status = super::super::FilterStatus<ResultStatus>;

#[armonik_macros::message("armonik.api.grpc.v1.results.FilterField")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(oneof = "value_condition")]
pub enum Condition {
    /// No condition. A `FilterField` that names a field but no condition cannot be evaluated;
    /// reading it as the first condition holding its defaults would filter on something else.
    #[default]
    Invalid,
    #[armonik(rename = "filter_string")]
    String(FilterString),
    #[armonik(rename = "filter_date")]
    Date(FilterDate),
    #[armonik(rename = "filter_array")]
    Array(FilterArray),
    #[armonik(rename = "filter_status")]
    Status(Status),
    #[armonik(rename = "filter_number")]
    Number(FilterNumber),
}
