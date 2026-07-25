use super::TaskRequestHeader;

#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.InitTaskRequest", oneof = "type")]
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
