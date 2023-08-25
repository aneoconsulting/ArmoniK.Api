use super::super::DataChunk;

use crate::api::v3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Request {
    Init {
        communication_token: String,
        key: String,
    },
    DataChunk {
        communication_token: String,
        chunk: DataChunk,
    },
    LastResult {
        communication_token: String,
    },
}

impl Default for Request {
    fn default() -> Self {
        Self::Init {
            communication_token: Default::default(),
            key: Default::default(),
        }
    }
}

impl From<Request> for v3::agent::Result {
    fn from(value: Request) -> Self {
        match value {
            Request::Init {
                communication_token,
                key,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::result::Type::Init(v3::InitKeyedDataStream {
                    r#type: Some(v3::init_keyed_data_stream::Type::Key(key)),
                })),
            },
            Request::DataChunk {
                communication_token,
                chunk,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::result::Type::Data(chunk.into())),
            },
            Request::LastResult {
                communication_token,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::result::Type::Init(v3::InitKeyedDataStream {
                    r#type: Some(v3::init_keyed_data_stream::Type::LastResult(true)),
                })),
            },
        }
    }
}

impl From<v3::agent::Result> for Request {
    fn from(value: v3::agent::Result) -> Self {
        match value.r#type {
            Some(v3::agent::result::Type::Data(chunk)) => Self::DataChunk {
                communication_token: value.communication_token,
                chunk: chunk.into(),
            },
            Some(v3::agent::result::Type::Init(init)) => match init.r#type {
                Some(v3::init_keyed_data_stream::Type::Key(key)) => Self::Init {
                    communication_token: value.communication_token,
                    key,
                },
                Some(v3::init_keyed_data_stream::Type::LastResult(_)) => Self::LastResult {
                    communication_token: value.communication_token,
                },
                None => Self::Init {
                    communication_token: value.communication_token,
                    key: Default::default(),
                },
            },
            None => Self::Init {
                communication_token: value.communication_token,
                key: Default::default(),
            },
        }
    }
}

super::super::impl_convert!(req Request : v3::agent::Result);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Response {
    Ok {
        communication_token: String,
    },
    Error {
        communication_token: String,
        error: String,
    },
}

impl Default for Response {
    fn default() -> Self {
        Self::Error {
            communication_token: Default::default(),
            error: Default::default(),
        }
    }
}

impl From<Response> for v3::agent::ResultReply {
    fn from(value: Response) -> Self {
        match value {
            Response::Ok {
                communication_token,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::result_reply::Type::Ok(v3::Empty {})),
            },
            Response::Error {
                communication_token,
                error,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::result_reply::Type::Error(error)),
            },
        }
    }
}

impl From<v3::agent::ResultReply> for Response {
    fn from(value: v3::agent::ResultReply) -> Self {
        match value.r#type {
            Some(v3::agent::result_reply::Type::Ok(_)) => Self::Ok {
                communication_token: value.communication_token,
            },
            Some(v3::agent::result_reply::Type::Error(error)) => Self::Error {
                communication_token: value.communication_token,
                error,
            },
            None => Self::Error {
                communication_token: value.communication_token,
                error: Default::default(),
            },
        }
    }
}

super::super::impl_convert!(req Response : v3::agent::ResultReply);
