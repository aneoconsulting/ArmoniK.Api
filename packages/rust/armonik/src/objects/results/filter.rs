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

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(
    message = "armonik.api.grpc.v1.results.FilterField",
    oneof = "value_condition"
)]
pub enum Condition {
    /// No condition: `value_condition` was left unset.
    ///
    /// A `FilterField` naming a field but no condition cannot be evaluated, so this is what a peer
    /// that set no member decodes to. Without it the absence read as the first condition holding
    /// its defaults, which is a filter over a different set rather than no filter at all.
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
