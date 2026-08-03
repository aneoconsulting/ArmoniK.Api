use super::super::{
    FilterArray, FilterBoolean, FilterDate, FilterDuration, FilterNumber, FilterString, TaskStatus,
};

use crate::impl_filter;

impl_filter!(
    Filter[super::Field, Condition]:
    protos[
        "armonik.api.grpc.v1.tasks.Filters",
        "armonik.api.grpc.v1.tasks.FiltersAnd",
        "armonik.api.grpc.v1.tasks.FilterField"
    ]
);

#[armonik_macros::alias("armonik.api.grpc.v1.tasks.FilterStatus")]
pub type Status = super::super::FilterStatus<TaskStatus>;

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.tasks.FilterField",
    oneof = "value_condition"
)]
pub enum Condition {
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

impl Default for Condition {
    fn default() -> Self {
        Self::String(Default::default())
    }
}
