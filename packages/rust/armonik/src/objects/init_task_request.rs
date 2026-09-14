use super::TaskRequestHeader;

#[armonik_macros::message("armonik.api.grpc.v1.InitTaskRequest")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum InitTaskRequest {
    /// No member set, which a defaulted `Header` is not.
    #[default]
    Invalid,
    Header(TaskRequestHeader),
    #[armonik(present)]
    LastTask,
}
