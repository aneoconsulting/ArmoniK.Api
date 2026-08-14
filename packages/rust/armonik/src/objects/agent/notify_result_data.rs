/// One result to notify: the session it belongs to, and its own id.
#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[armonik(message = "armonik.api.grpc.v1.agent.NotifyResultDataRequest.ResultIdentifier")]
pub struct ResultIdentifier {
    pub session_id: String,
    pub result_id: String,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.agent.NotifyResultDataRequest")]
pub struct Request {
    pub communication_token: String,
    #[armonik(rename = "ids")]
    pub results: Vec<ResultIdentifier>,
}

impl Request {
    /// Notify results that all belong to one session, which is what every caller does.
    ///
    /// The pairs carry a session each because the proto says so, not because a caller would want to
    /// vary it. This spells the common case; the field is there for the one that does not, and for
    /// reading a request off the wire without losing anything.
    pub fn in_session(
        communication_token: impl Into<String>,
        session_id: impl Into<String>,
        result_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let session_id = session_id.into();
        Self {
            communication_token: communication_token.into(),
            results: result_ids
                .into_iter()
                .map(|result_id| ResultIdentifier {
                    session_id: session_id.clone(),
                    result_id: result_id.into(),
                })
                .collect(),
        }
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.agent.NotifyResultDataResponse")]
pub struct Response {
    pub result_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::{Request, ResultIdentifier};

    /// prost-derived reference of `NotifyResultDataRequest` and its `ResultIdentifier` pairs, as an
    /// independent codec: the multi-pair fixture below is encoded through it and decoded through
    /// ours.
    #[derive(Clone, PartialEq, Message)]
    struct RefIdentifier {
        #[prost(string, tag = "1")]
        session_id: String,
        #[prost(string, tag = "2")]
        result_id: String,
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefRequest {
        #[prost(message, repeated, tag = "1")]
        ids: Vec<RefIdentifier>,
        #[prost(string, tag = "4")]
        communication_token: String,
    }

    /// Every pair keeps its own session.
    ///
    /// The request used to flatten the pairs into one shared session id, keeping the first non-empty
    /// one, which lost the second session here: a server acting on the decoded request marked `r2`
    /// available under `s1`. The empty session on `r0` was lost the same way, in the other
    /// direction, since encoding replicated the shared id into every pair.
    #[test]
    fn each_pair_keeps_its_own_session() {
        let reference = RefRequest {
            ids: vec![
                RefIdentifier {
                    session_id: String::new(),
                    result_id: "r0".to_owned(),
                },
                RefIdentifier {
                    session_id: "s1".to_owned(),
                    result_id: "r1".to_owned(),
                },
                RefIdentifier {
                    session_id: "s2".to_owned(),
                    result_id: "r2".to_owned(),
                },
            ],
            communication_token: "tok".to_owned(),
        };
        let request = Request::decode(reference.encode_to_vec().as_slice()).expect("decodes");
        assert_eq!(
            request.results,
            [
                ResultIdentifier {
                    session_id: String::new(),
                    result_id: "r0".to_owned()
                },
                ResultIdentifier {
                    session_id: "s1".to_owned(),
                    result_id: "r1".to_owned()
                },
                ResultIdentifier {
                    session_id: "s2".to_owned(),
                    result_id: "r2".to_owned()
                },
            ]
        );
        assert_eq!(request.communication_token, "tok");
        assert_eq!(
            RefRequest::decode(request.encode_to_vec().as_slice()).expect("decodes"),
            reference
        );
    }

    /// The session id survives a request with no result, which the flattening dropped: it only ever
    /// reached the wire inside a pair, and there was no pair to put it in.
    #[test]
    fn a_request_with_no_result_still_names_its_session() {
        let request = Request::in_session("tok", "s1", Vec::<String>::new());
        assert!(request.results.is_empty());

        let request = Request::in_session("tok", "s1", ["r1"]);
        let back = Request::decode(request.encode_to_vec().as_slice()).expect("decodes");
        assert_eq!(back, request);
        assert_eq!(back.results[0].session_id, "s1");
    }
}
