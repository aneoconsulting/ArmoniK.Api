use super::super::SessionStatus;

/// Status selector of the filter.
#[armonik_macros::oneof("armonik.api.grpc.v1.submitter.SessionFilter.statuses")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SessionFilterStatuses {
    /// No selector. Distinct from `Exclude([])`, which is a constraint that happens to be vacuous.
    #[default]
    Invalid,
    /// Select the sessions whose status is one of these.
    #[armonik(rename = "included", inlined)]
    Include(Vec<SessionStatus>),
    /// Select the sessions whose status is none of these.
    #[armonik(rename = "excluded", inlined)]
    Exclude(Vec<SessionStatus>),
}

#[armonik_macros::message("armonik.api.grpc.v1.submitter.SessionFilter")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionFilter {
    #[armonik(rename = "sessions")]
    pub ids: Vec<String>,
    pub statuses: SessionFilterStatuses,
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::*;

    /// Independent prost-derived reference, with the oneof spelled out as optional fields.
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
