use super::TaskRequestHeader;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.InitTaskRequest")]
pub enum InitTaskRequest {
    /// No member set, which a defaulted `Header` is not.
    #[default]
    Invalid,
    Header(TaskRequestHeader),
    #[armonik(present)]
    LastTask,
}
