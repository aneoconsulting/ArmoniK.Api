use super::TaskRequestHeader;

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.InitTaskRequest")]
pub enum InitTaskRequest {
    Header(TaskRequestHeader),
    #[armonik(present)]
    LastTask,
}

impl Default for InitTaskRequest {
    fn default() -> Self {
        Self::Header(Default::default())
    }
}
