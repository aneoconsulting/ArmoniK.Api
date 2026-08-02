use super::super::SessionStatus;

/// Status selector of the filter.
///
/// The `Include`/`Exclude` variants map to the *opposite* proto members
/// (`excluded`/`included`), reproducing the historical conversions exactly.
#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.submitter.SessionFilter",
    oneof = "statuses"
)]
pub enum SessionFilterStatuses {
    #[armonik(
        rename = "excluded",
        with = "crate::codec::adapters::Wrapper<1>",
        absorbs = "armonik.api.grpc.v1.submitter.SessionFilter.StatusesRequest"
    )]
    Include(Vec<SessionStatus>),
    #[armonik(
        rename = "included",
        with = "crate::codec::adapters::Wrapper<1>",
        absorbs = "armonik.api.grpc.v1.submitter.SessionFilter.StatusesRequest"
    )]
    Exclude(Vec<SessionStatus>),
}

impl Default for SessionFilterStatuses {
    fn default() -> Self {
        Self::Exclude(Default::default())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.SessionFilter")]
pub struct SessionFilter {
    #[armonik(rename = "sessions")]
    pub ids: Vec<String>,
    pub statuses: SessionFilterStatuses,
}
