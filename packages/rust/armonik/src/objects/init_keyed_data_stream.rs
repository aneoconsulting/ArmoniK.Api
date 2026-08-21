#[armonik_macros::message("armonik.api.grpc.v1.InitKeyedDataStream")]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InitKeyedDataStream {
    /// No member set, which an empty `Key` is not.
    #[default]
    Invalid,
    Key(String),
    #[armonik(present)]
    LastResult,
}
