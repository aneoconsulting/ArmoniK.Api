#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.InitKeyedDataStream")]
pub enum InitKeyedDataStream {
    /// No member set.
    ///
    /// The absence used to decode to an empty `Key`, which is a key rather than the lack of one.
    #[default]
    Invalid,
    Key(String),
    #[armonik(present)]
    LastResult,
}
