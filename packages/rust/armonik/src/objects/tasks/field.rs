use super::super::TaskOptionField;

#[armonik_macros::enumeration]
#[derive(Debug, Clone, Copy)]
#[armonik(transparent, message = "armonik.api.grpc.v1.tasks.TaskSummaryField")]
pub enum SummaryField {
    TaskId,
    SessionId,
    OwnerPodId,
    InitialTaskId,
    Status,
    CreatedAt,
    SubmittedAt,
    StartedAt,
    EndedAt,
    CreationToEndDuration,
    ProcessingToEndDuration,
    ReceivedToEndDuration,
    PodTtl,
    PodHostname,
    ReceivedAt,
    AcquiredAt,
    ProcessedAt,
    FetchedAt,
    Error,
    PayloadId,
    CreatedBy,
    /// Unspecified (zero) or a field unknown to this crate version.
    Unknown(UnknownSummaryField),
}

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.tasks.TaskField")]
pub enum Field {
    #[armonik(rename = "task_summary_field")]
    Summary(SummaryField),
    #[armonik(rename = "task_option_field")]
    Option(TaskOptionField),
    #[armonik(
        rename = "task_option_generic_field",
        with = "crate::codec::adapters::Wrapper",
        absorbs = "armonik.api.grpc.v1.tasks.TaskOptionGenericField"
    )]
    OptionGeneric(String),
}

impl Default for Field {
    fn default() -> Self {
        Self::Summary(Default::default())
    }
}
