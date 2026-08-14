use super::TaskRequestHeader;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.InitTaskRequest")]
pub enum InitTaskRequest {
    /// No member set.
    ///
    /// The absence used to decode to a default `Header`, which reads as a task with no options
    /// rather than as a message that names neither.
    #[default]
    Invalid,
    Header(TaskRequestHeader),
    #[armonik(present)]
    LastTask,
}
