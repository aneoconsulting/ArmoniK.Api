#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[armonik(message = "armonik.api.grpc.v1.Output")]
pub enum Output {
    /// No outcome: the oneof was left unset.
    ///
    /// This is the one place the distinction is load-bearing rather than tidy. `Ok` is a
    /// [`present`](armonik_macros::message#present) variant, so it carries nothing beyond being
    /// set, and an absent oneof used to be indistinguishable from it: a peer that said nothing was
    /// read as one that said the task succeeded. [`crate::tasks::Output`], whose wire impl is
    /// hand-written, already refused that reading by defaulting to an empty error.
    #[default]
    Invalid,
    #[armonik(present)]
    Ok,
    #[armonik(inline)]
    Error { details: String },
}
