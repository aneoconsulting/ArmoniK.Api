use super::super::{
    FilterArray, FilterBoolean, FilterDate, FilterDuration, FilterNumber, FilterString,
    SessionStatus,
};

use crate::impl_filter;

impl_filter!(
    Filter[super::Field, Condition]:
    protos[
        "armonik.api.grpc.v1.sessions.Filters",
        "armonik.api.grpc.v1.sessions.FiltersAnd",
        "armonik.api.grpc.v1.sessions.FilterField"
    ]
);

#[armonik_macros::alias("armonik.api.grpc.v1.sessions.FilterStatus")]
pub type Status = super::super::FilterStatus<SessionStatus>;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(
    message = "armonik.api.grpc.v1.sessions.FilterField",
    oneof = "value_condition"
)]
pub enum Condition {
    /// No condition. A `FilterField` that names a field but no condition cannot be evaluated;
    /// reading it as the first condition holding its defaults would filter on something else.
    #[default]
    Invalid,
    #[armonik(rename = "filter_string")]
    String(FilterString),
    #[armonik(rename = "filter_number")]
    Number(FilterNumber),
    #[armonik(rename = "filter_boolean")]
    Boolean(FilterBoolean),
    #[armonik(rename = "filter_status")]
    Status(Status),
    #[armonik(rename = "filter_date")]
    Date(FilterDate),
    #[armonik(rename = "filter_duration")]
    Duration(FilterDuration),
    #[armonik(rename = "filter_array")]
    Array(FilterArray),
}
