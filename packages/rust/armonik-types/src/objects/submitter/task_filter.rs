use super::super::TaskStatus;

/// Task selector of the filter.
#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter", oneof = "ids")]
pub enum TaskFilterIds {
    /// Select the tasks from their session IDs.
    #[armonik(
        rename = "session",
        with = "crate::codec::adapters::VecWrapper<1>",
        absorbs = "armonik.api.grpc.v1.submitter.TaskFilter.IdsRequest"
    )]
    Sessions(Vec<String>),
    /// Select the tasks from their task IDs.
    #[armonik(
        rename = "task",
        with = "crate::codec::adapters::VecWrapper<1>",
        absorbs = "armonik.api.grpc.v1.submitter.TaskFilter.IdsRequest"
    )]
    Tasks(Vec<String>),
}

impl Default for TaskFilterIds {
    fn default() -> Self {
        Self::Sessions(Default::default())
    }
}

/// Status selector of the filter.
///
/// The `Include`/`Exclude` variants map to the *opposite* proto members
/// (`excluded`/`included`), reproducing the historical conversions exactly.
#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.submitter.TaskFilter",
    oneof = "statuses"
)]
pub enum TaskFilterStatuses {
    #[armonik(
        rename = "excluded",
        with = "crate::codec::adapters::VecWrapper<1>",
        absorbs = "armonik.api.grpc.v1.submitter.TaskFilter.StatusesRequest"
    )]
    Include(Vec<TaskStatus>),
    #[armonik(
        rename = "included",
        with = "crate::codec::adapters::VecWrapper<1>",
        absorbs = "armonik.api.grpc.v1.submitter.TaskFilter.StatusesRequest"
    )]
    Exclude(Vec<TaskStatus>),
}

impl Default for TaskFilterStatuses {
    fn default() -> Self {
        Self::Exclude(Default::default())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.TaskFilter")]
pub struct TaskFilter {
    pub ids: TaskFilterIds,
    pub statuses: TaskFilterStatuses,
}
