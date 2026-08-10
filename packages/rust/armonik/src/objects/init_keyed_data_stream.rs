#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
