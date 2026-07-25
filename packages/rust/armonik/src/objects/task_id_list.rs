use crate::api::v3;

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.TaskIdList")]
pub struct TaskIdList {
    pub task_ids: Vec<String>,
}

super::impl_convert!(
    struct TaskIdList = v3::TaskIdList {
        task_ids,
    }
);
