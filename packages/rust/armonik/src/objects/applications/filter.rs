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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(
    message = "armonik.api.grpc.v1.applications.FilterField",
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
}
