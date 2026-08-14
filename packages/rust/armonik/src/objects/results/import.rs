use std::collections::HashMap;

use super::Raw;

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.ImportResultsDataRequest")]
pub struct Request {
    pub session_id: String,
    /// The opaque storage id to import into each result, keyed by result id.
    #[armonik(
        with = "crate::codec::adapters::PairMap",
        absorbs = "armonik.api.grpc.v1.results.ImportResultsDataRequest.ResultOpaqueId"
    )]
    pub results: HashMap<String, bytes::Bytes>,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.results.ImportResultsDataResponse")]
pub struct Response {
    pub results: Vec<Raw>,
}

impl Response {
    /// The updated results by name.
    ///
    /// A view rather than the field, and borrowed rather than owned, because `name` is not a key:
    /// nothing on the wire makes it unique, so this map can hold fewer entries than `results`. The
    /// response used to *be* this map, which meant a round trip could drop a result and no caller
    /// could tell. It could not be correlated with the request either, which is keyed by result id.
    pub fn by_name(&self) -> HashMap<&str, &Raw> {
        self.results
            .iter()
            .map(|raw| (raw.name.as_str(), raw))
            .collect()
    }

    /// The updated results by result id, which is what [`Request::results`] is keyed by.
    pub fn by_result_id(&self) -> HashMap<&str, &Raw> {
        self.results
            .iter()
            .map(|raw| (raw.result_id.as_str(), raw))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Raw, Response};

    fn raw(result_id: &str, name: &str) -> Raw {
        Raw {
            result_id: String::from(result_id),
            name: String::from(name),
            ..Default::default()
        }
    }

    /// Two results may share a name, and the response keeps both. Keying the field by name lost one
    /// of them, silently, and left the survivor unmatchable against the request that asked for it.
    #[test]
    fn two_results_sharing_a_name_both_survive() {
        let response = Response {
            results: vec![raw("r1", "shared"), raw("r2", "shared")],
        };

        assert_eq!(response.results.len(), 2);
        assert_eq!(response.by_name().len(), 1);
        assert_eq!(response.by_result_id().len(), 2);
        assert_eq!(response.by_result_id()["r1"].name, "shared");
        assert_eq!(response.by_result_id()["r2"].name, "shared");
    }
}
