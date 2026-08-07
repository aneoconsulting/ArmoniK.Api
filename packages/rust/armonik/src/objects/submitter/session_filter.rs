use super::super::SessionStatus;

/// Status selector of the filter.
#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.submitter.SessionFilter",
    oneof = "statuses"
)]
pub enum SessionFilterStatuses {
    /// Select the sessions whose status is one of these.
    #[armonik(
        rename = "included",
        with = "crate::codec::adapters::Wrapper<1>",
        absorbs = "armonik.api.grpc.v1.submitter.SessionFilter.StatusesRequest"
    )]
    Include(Vec<SessionStatus>),
    /// Select the sessions whose status is none of these.
    #[armonik(
        rename = "excluded",
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

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.submitter.SessionFilter")]
pub struct SessionFilter {
    #[armonik(rename = "sessions")]
    pub ids: Vec<String>,
    pub statuses: SessionFilterStatuses,
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::*;

    /// prost-derived reference of `SessionFilter` (the generated type no longer
    /// exists), with the oneof spelled out as optional fields.
    #[derive(Clone, PartialEq, Message)]
    struct RefFilter {
        #[prost(string, repeated, tag = "1")]
        sessions: Vec<String>,
        #[prost(message, optional, tag = "4")]
        included: Option<RefStatuses>,
        #[prost(message, optional, tag = "5")]
        excluded: Option<RefStatuses>,
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefStatuses {
        #[prost(int32, repeated, tag = "1")]
        statuses: Vec<i32>,
    }

    fn statuses(status: SessionStatus) -> Option<RefStatuses> {
        Some(RefStatuses {
            statuses: vec![i32::from(status)],
        })
    }

    /// Each selector variant names its own proto member, in both directions.
    #[test]
    fn selectors_bind_their_own_proto_member() {
        for (ours, theirs) in [
            (
                SessionFilterStatuses::Include(vec![SessionStatus::Running]),
                RefFilter {
                    included: statuses(SessionStatus::Running),
                    ..Default::default()
                },
            ),
            (
                SessionFilterStatuses::Exclude(vec![SessionStatus::Cancelled]),
                RefFilter {
                    excluded: statuses(SessionStatus::Cancelled),
                    ..Default::default()
                },
            ),
        ] {
            let filter = SessionFilter {
                ids: vec![String::from("session-id")],
                statuses: ours,
            };
            let expected = RefFilter {
                sessions: vec![String::from("session-id")],
                ..theirs
            };

            assert_eq!(
                RefFilter::decode(filter.encode_to_vec().as_slice()).unwrap(),
                expected
            );
            assert_eq!(
                SessionFilter::decode(expected.encode_to_vec().as_slice()).unwrap(),
                filter
            );
        }
    }
}
