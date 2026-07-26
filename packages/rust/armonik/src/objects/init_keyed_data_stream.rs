#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.InitKeyedDataStream")]
pub enum InitKeyedDataStream {
    Key(String),
    #[armonik(present)]
    LastResult,
}

impl Default for InitKeyedDataStream {
    fn default() -> Self {
        Self::Key(Default::default())
    }
}
