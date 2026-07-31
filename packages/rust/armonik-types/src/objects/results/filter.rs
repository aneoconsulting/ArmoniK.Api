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

#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.results.FilterField",
    oneof = "value_condition"
)]
pub enum Condition {
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

impl Default for Condition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}
