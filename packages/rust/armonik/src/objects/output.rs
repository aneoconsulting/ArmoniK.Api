#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[armonik(message = "armonik.api.grpc.v1.Output")]
pub enum Output {
    /// No member set. Distinct from [`Ok`](Self::Ok), which carries nothing but *is* set: a peer
    /// that reports no outcome is not a peer that reports success.
    #[default]
    Invalid,
    #[armonik(present)]
    Ok,
    #[armonik(inline)]
    Error { details: String },
}
