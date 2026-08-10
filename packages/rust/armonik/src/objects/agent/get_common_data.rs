#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.agent.DataRequest")]
pub struct Request {
    pub communication_token: String,
    pub result_id: String,
}

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[armonik(message = "armonik.api.grpc.v1.agent.DataResponse")]
pub struct Response {
    pub result_id: String,
}
