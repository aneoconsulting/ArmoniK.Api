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
    /// Notify results that all belong to one session, which is what every caller does. Build
    /// [`results`](Self::results) directly to vary the session per result.
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

    /// Independent prost-derived reference: the fixture below is encoded through it, decoded
    /// through ours.
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

    /// Every pair keeps its own session, including the empty one, and pairs may disagree.
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

    /// A request with no result carries no session either: the session only reaches the wire
    /// inside a pair.
    #[test]
    fn the_session_reaches_the_wire_only_inside_a_pair() {
        let request = Request::in_session("tok", "s1", Vec::<String>::new());
        assert!(request.results.is_empty());

        let request = Request::in_session("tok", "s1", ["r1"]);
        let back = Request::decode(request.encode_to_vec().as_slice()).expect("decodes");
        assert_eq!(back, request);
        assert_eq!(back.results[0].session_id, "s1");
    }
}
