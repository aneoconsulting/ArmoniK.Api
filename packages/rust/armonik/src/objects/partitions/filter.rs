use super::super::{FilterArray, FilterBoolean, FilterNumber, FilterString};

use crate::impl_filter;

impl_filter!(
    Filter[super::Field, Condition]:
    protos[
        "armonik.api.grpc.v1.partitions.Filters",
        "armonik.api.grpc.v1.partitions.FiltersAnd",
        "armonik.api.grpc.v1.partitions.FilterField"
    ]
);

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[armonik(
    message = "armonik.api.grpc.v1.partitions.FilterField",
    oneof = "value_condition"
)]
pub enum Condition {
    #[armonik(rename = "filter_string")]
    String(FilterString),
    #[armonik(rename = "filter_number")]
    Number(FilterNumber),
    #[armonik(rename = "filter_boolean")]
    Boolean(FilterBoolean),
    #[armonik(rename = "filter_array")]
    Array(FilterArray),
}

impl Default for Condition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}
