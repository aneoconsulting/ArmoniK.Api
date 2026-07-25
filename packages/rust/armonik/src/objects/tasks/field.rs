use super::super::TaskOptionField;

/// Represents every available field in a Task.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Enum,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(transparent, message = "armonik.api.grpc.v1.tasks.TaskSummaryField")]
pub enum SummaryField {
    /// The task ID.
    #[default]
    TaskId,
    /// The session ID.
    SessionId,
    /// The owner pod ID.
    OwnerPodId,
    /// The initial task ID. Set when a task is submitted independently of retries.
    InitialTaskId,
    /// The task status.
    Status,
    /// The task creation date.
    CreatedAt,
    /// The task submission date.
    SubmittedAt,
    /// The task start date.
    StartedAt,
    /// The task end date.
    EndedAt,
    /// The task duration. Between the creation date and the end date.
    CreationToEndDuration,
    /// The task calculated duration. Between the start date and the end date.
    ProcessingToEndDuration,
    /// The task calculated duration. Between the received date and the end date.
    ReceivedToEndDuration,
    /// The pod TTL (Time To Live).
    PodTtl,
    /// The hostname of the container running the task.
    PodHostname,
    /// When the task is received by the agent.
    ReceivedAt,
    /// When the task is acquired by the agent.
    AcquiredAt,
    /// When the task is processed by the agent.
    ProcessedAt,
    /// When task data are fetched by the agent.
    FetchedAt,
    /// The error message. Only set if task have failed.
    Error,
    /// The ID of the Result that is used as a payload for this task.
    PayloadId,
    /// The ID of the Result that is used as a payload for this task.
    CreatedBy,
    /// Unspecified (zero) or a field unknown to this crate version.
    Other(OtherSummaryField),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.tasks.TaskField", oneof = "field")]
pub enum Field {
    /// The task summary field.
    #[armonik(rename = "task_summary_field")]
    Summary(SummaryField),
    /// The task option field.
    #[armonik(rename = "task_option_field")]
    Option(TaskOptionField),
    /// Represents a generic field in a task option.
    #[armonik(
        rename = "task_option_generic_field",
        with = "crate::codec::adapters::StringWrapper<1>"
    )]
    OptionGeneric(String),
}

impl Default for Field {
    fn default() -> Self {
        Self::Summary(Default::default())
    }
}
