use super::super::DataChunk;

use crate::api::v3;

#[derive(Debug, Clone)]
pub enum Data {
    DataChunk {
        communication_token: String,
        key: Option<String>,
        chunk: DataChunk,
    },
    Error {
        communication_token: String,
        key: Option<String>,
        error: String,
    },
}

impl Default for Data {
    fn default() -> Self {
        Self::Error {
            communication_token: Default::default(),
            key: Default::default(),
            error: Default::default(),
        }
    }
}

impl From<Data> for v3::agent::DataReply {
    fn from(value: Data) -> Self {
        match value {
            Data::DataChunk {
                communication_token,
                key: Some(key),
                chunk,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::data_reply::Type::Init(
                    v3::agent::data_reply::Init {
                        key,
                        has_result: Some(v3::agent::data_reply::init::HasResult::Data(
                            chunk.into(),
                        )),
                    },
                )),
            },
            Data::Error {
                communication_token,
                key: Some(key),
                error,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::data_reply::Type::Init(
                    v3::agent::data_reply::Init {
                        key,
                        has_result: Some(v3::agent::data_reply::init::HasResult::Error(error)),
                    },
                )),
            },
            Data::DataChunk {
                communication_token,
                key: None,
                chunk,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::data_reply::Type::Data(chunk.into())),
            },
            Data::Error {
                communication_token,
                key: None,
                error,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::data_reply::Type::Error(error)),
            },
        }
    }
}

impl From<v3::agent::DataReply> for Data {
    fn from(value: v3::agent::DataReply) -> Self {
        match value.r#type {
            Some(v3::agent::data_reply::Type::Init(init)) => match init.has_result {
                Some(v3::agent::data_reply::init::HasResult::Data(chunk)) => Self::DataChunk {
                    communication_token: value.communication_token,
                    key: Some(init.key),
                    chunk: chunk.into(),
                },
                Some(v3::agent::data_reply::init::HasResult::Error(error)) => Self::Error {
                    communication_token: value.communication_token,
                    key: Some(init.key),
                    error,
                },
                None => Self::Error {
                    communication_token: value.communication_token,
                    key: Some(init.key),
                    error: Default::default(),
                },
            },
            Some(v3::agent::data_reply::Type::Data(chunk)) => Self::DataChunk {
                communication_token: value.communication_token,
                key: None,
                chunk: chunk.into(),
            },
            Some(v3::agent::data_reply::Type::Error(error)) => Self::Error {
                communication_token: value.communication_token,
                key: None,
                error,
            },
            None => Default::default(),
        }
    }
}

super::super::impl_convert!(req Data : v3::agent::DataReply);
