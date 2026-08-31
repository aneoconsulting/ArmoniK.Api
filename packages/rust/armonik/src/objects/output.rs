#[armonik_macros::message("armonik.api.grpc.v1.Output")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Output {
    /// No member set. Distinct from [`Ok`](Self::Ok), which carries nothing but *is* set: a peer
    /// that reports no outcome is not a peer that reports success.
    #[default]
    Invalid,
    #[armonik(present)]
    Ok,
    #[armonik(inlined)]
    Error { details: String },
}
