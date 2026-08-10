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

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[armonik(
    message = "armonik.api.grpc.v1.applications.FilterField",
    oneof = "value_condition"
)]
pub enum Condition {
    #[armonik(rename = "filter_string")]
    String(FilterString),
}

impl Default for Condition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}
