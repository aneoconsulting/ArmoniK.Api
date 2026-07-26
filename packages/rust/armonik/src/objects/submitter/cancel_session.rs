/// Request for cancelling a session, standing for the `Session` message the
/// stubs use.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Request {
    pub session_id: String,
}

impl From<Request> for crate::Session {
    fn from(value: Request) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

impl From<crate::Session> for Request {
    fn from(value: crate::Session) -> Self {
        Self {
            session_id: value.session_id,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.Empty")]
pub struct Response {}
