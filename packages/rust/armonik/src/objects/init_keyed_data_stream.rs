#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.InitKeyedDataStream")]
pub enum InitKeyedDataStream {
    /// No member set, which an empty `Key` is not.
    #[default]
    Invalid,
    Key(String),
    #[armonik(present)]
    LastResult,
}
