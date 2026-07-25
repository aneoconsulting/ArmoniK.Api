use super::super::{DataChunk, InitTaskRequest, TaskOptions};
use crate::utils::IntoCollection;

use crate::api::v3;
use crate::codec::{adapters::VecWrapper, message as message_codec, ProtoAdapter, ProtoField};

#[derive(Debug, Clone, Default, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateTaskRequest.InitRequest")]
pub struct InitRequest {
    pub task_options: Option<TaskOptions>,
}

super::super::impl_convert!(
    struct InitRequest = v3::agent::create_task_request::InitRequest {
        option task_options,
    }
);

/// The `CreateTaskRequest` message: a oneof (tags 1-3) plus a sibling
/// `communication_token = 4`, flattened into token-carrying variants.
///
/// The `prost::Message` implementation is hand-written: the sibling field
/// breaks the per-field merge model (the token can precede the oneof members
/// on the wire), so the provided [`prost::Message::merge`] is overridden to
/// buffer it. This type is never nested inside another message — enforced by
/// not implementing `ProtoField` for it — so `merge_field`'s best-effort
/// handling of the token is never exercised by nested decoding.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Request {
    #[default]
    Invalid,
    InitRequest {
        communication_token: String,
        request: InitRequest,
    },
    InitTaskRequest {
        communication_token: String,
        request: InitTaskRequest,
    },
    DataChunk {
        communication_token: String,
        chunk: DataChunk,
    },
}

impl Request {
    fn token_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Invalid => None,
            Self::InitRequest {
                communication_token,
                ..
            }
            | Self::InitTaskRequest {
                communication_token,
                ..
            }
            | Self::DataChunk {
                communication_token,
                ..
            } => Some(communication_token),
        }
    }

    /// Merge one oneof member into the variant; the caller re-applies the
    /// token afterwards.
    fn merge_variant(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut impl prost::bytes::Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1 => {
                let mut request = if let Self::InitRequest { request, .. } = self {
                    std::mem::take(request)
                } else {
                    InitRequest::wire_default()
                };
                message_codec::merge(wire_type, &mut request, buf, ctx)?;
                *self = Self::InitRequest {
                    communication_token: String::new(),
                    request,
                };
                Ok(())
            }
            2 => {
                let mut request = if let Self::InitTaskRequest { request, .. } = self {
                    std::mem::take(request)
                } else {
                    InitTaskRequest::wire_default()
                };
                message_codec::merge(wire_type, &mut request, buf, ctx)?;
                *self = Self::InitTaskRequest {
                    communication_token: String::new(),
                    request,
                };
                Ok(())
            }
            3 => {
                let mut chunk = if let Self::DataChunk { chunk, .. } = self {
                    std::mem::take(chunk)
                } else {
                    DataChunk::wire_default()
                };
                message_codec::merge(wire_type, &mut chunk, buf, ctx)?;
                *self = Self::DataChunk {
                    communication_token: String::new(),
                    chunk,
                };
                Ok(())
            }
            _ => unreachable!("only oneof member tags are routed here"),
        }
    }
}

impl prost::Message for Request {
    fn encode_raw(&self, buf: &mut impl prost::bytes::BufMut) {
        let token = match self {
            Self::Invalid => return,
            Self::InitRequest {
                communication_token,
                request,
            } => {
                message_codec::encode(1, request, buf);
                communication_token
            }
            Self::InitTaskRequest {
                communication_token,
                request,
            } => {
                message_codec::encode(2, request, buf);
                communication_token
            }
            Self::DataChunk {
                communication_token,
                chunk,
            } => {
                message_codec::encode(3, chunk, buf);
                communication_token
            }
        };
        if !token.is_empty() {
            ProtoField::encode_field(4, token, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut impl prost::bytes::Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1..=3 => {
                let token = self.token_mut().map(std::mem::take);
                self.merge_variant(tag, wire_type, buf, ctx)?;
                if let (Some(token), Some(slot)) = (token, self.token_mut()) {
                    *slot = token;
                }
                Ok(())
            }
            4 => {
                let mut token = String::new();
                ProtoField::merge_field(wire_type, &mut token, buf, ctx)?;
                if let Some(slot) = self.token_mut() {
                    *slot = token;
                }
                Ok(())
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    /// Top-level decode entry: buffers the sibling token so that any wire
    /// field order produces the same value. A token without any oneof member
    /// decodes to `Invalid` (token dropped), like the historical conversion.
    fn merge(&mut self, mut buf: impl prost::bytes::Buf) -> Result<(), prost::DecodeError>
    where
        Self: Sized,
    {
        let ctx = prost::encoding::DecodeContext::default();
        let mut token = self.token_mut().map(std::mem::take);
        while buf.has_remaining() {
            let (tag, wire_type) = prost::encoding::decode_key(&mut buf)?;
            match tag {
                1..=3 => self.merge_variant(tag, wire_type, &mut buf, ctx.clone())?,
                4 => {
                    ProtoField::merge_field(
                        wire_type,
                        token.get_or_insert_with(String::new),
                        &mut buf,
                        ctx.clone(),
                    )?;
                }
                _ => prost::encoding::skip_field(wire_type, tag, &mut buf, ctx.clone())?,
            }
        }
        if let (Some(token), Some(slot)) = (token, self.token_mut()) {
            *slot = token;
        }
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        let (payload_len, token) = match self {
            Self::Invalid => return 0,
            Self::InitRequest {
                communication_token,
                request,
            } => (message_codec::encoded_len(1, request), communication_token),
            Self::InitTaskRequest {
                communication_token,
                request,
            } => (message_codec::encoded_len(2, request), communication_token),
            Self::DataChunk {
                communication_token,
                chunk,
            } => (message_codec::encoded_len(3, chunk), communication_token),
        };
        let token_len = if token.is_empty() {
            0
        } else {
            ProtoField::encoded_len_field(4, token)
        };
        payload_len + token_len
    }

    fn clear(&mut self) {
        *self = Self::Invalid;
    }
}

impl From<Request> for v3::agent::CreateTaskRequest {
    fn from(value: Request) -> Self {
        match value {
            Request::Invalid => Self {
                communication_token: Default::default(),
                r#type: None,
            },
            Request::InitRequest {
                communication_token,
                request,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::create_task_request::Type::InitRequest(
                    request.into(),
                )),
            },
            Request::InitTaskRequest {
                communication_token,
                request,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::create_task_request::Type::InitTask(
                    request.into(),
                )),
            },
            Request::DataChunk {
                communication_token,
                chunk,
            } => Self {
                communication_token,
                r#type: Some(v3::agent::create_task_request::Type::TaskPayload(
                    chunk.into(),
                )),
            },
        }
    }
}

impl From<v3::agent::CreateTaskRequest> for Request {
    fn from(value: v3::agent::CreateTaskRequest) -> Self {
        match value.r#type {
            Some(v3::agent::create_task_request::Type::InitRequest(request)) => Self::InitRequest {
                communication_token: value.communication_token,
                request: request.into(),
            },
            Some(v3::agent::create_task_request::Type::InitTask(request)) => {
                Self::InitTaskRequest {
                    communication_token: value.communication_token,
                    request: request.into(),
                }
            }
            Some(v3::agent::create_task_request::Type::TaskPayload(chunk)) => Self::DataChunk {
                communication_token: value.communication_token,
                chunk: chunk.into(),
            },
            None => Self::Invalid,
        }
    }
}

super::super::impl_convert!(req Request : v3::agent::CreateTaskRequest);

#[derive(Debug, Clone, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(
    message = "armonik.api.grpc.v1.agent.CreateTaskReply.CreationStatus",
    oneof = "Status"
)]
pub enum Status {
    TaskInfo {
        /// Unique ID of the created task.
        task_id: String,
        /// Unique ID of the result that will be used as expected output. Results should already exist.
        expected_output_keys: Vec<String>,
        /// Unique ID of the result that will be used as data dependency. Results should already exist.
        data_dependencies: Vec<String>,
        /// Unique ID of the result that will be used as payload. Result associated to the payload is created implicitly.
        payload_id: String,
    },
    Error(String),
}

impl Default for Status {
    fn default() -> Self {
        Self::Error(Default::default())
    }
}

impl From<Status> for v3::agent::create_task_reply::CreationStatus {
    fn from(value: Status) -> Self {
        match value {
            Status::TaskInfo {
                task_id,
                expected_output_keys,
                data_dependencies,
                payload_id,
            } => Self {
                status: Some(
                    v3::agent::create_task_reply::creation_status::Status::TaskInfo(
                        v3::agent::create_task_reply::TaskInfo {
                            task_id,
                            expected_output_keys,
                            data_dependencies,
                            payload_id,
                        },
                    ),
                ),
            },
            Status::Error(msg) => Self {
                status: Some(v3::agent::create_task_reply::creation_status::Status::Error(msg)),
            },
        }
    }
}

impl From<v3::agent::create_task_reply::CreationStatus> for Status {
    fn from(value: v3::agent::create_task_reply::CreationStatus) -> Self {
        match value.status {
            Some(v3::agent::create_task_reply::creation_status::Status::TaskInfo(status)) => {
                Self::TaskInfo {
                    task_id: status.task_id,
                    expected_output_keys: status.expected_output_keys,
                    data_dependencies: status.data_dependencies,
                    payload_id: status.payload_id,
                }
            }
            Some(v3::agent::create_task_reply::creation_status::Status::Error(msg)) => {
                Self::Error(msg)
            }
            None => Default::default(),
        }
    }
}

/// The `CreateTaskReply` message: a oneof (tags 1-2, with the
/// `CreationStatusList` wrapper flattened into `Vec<Status>`) plus a sibling
/// `communication_token = 4`; hand-written like [`Request`] and for the same
/// reasons.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Response {
    Status {
        communication_token: String,
        statuses: Vec<Status>,
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

impl Response {
    fn token_mut(&mut self) -> &mut String {
        match self {
            Self::Status {
                communication_token,
                ..
            }
            | Self::Error {
                communication_token,
                ..
            } => communication_token,
        }
    }

    fn merge_variant(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut impl prost::bytes::Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1 => {
                let mut statuses = if let Self::Status { statuses, .. } = self {
                    std::mem::take(statuses)
                } else {
                    Vec::new()
                };
                <VecWrapper<1> as ProtoAdapter<Vec<Status>>>::merge_field(
                    wire_type,
                    &mut statuses,
                    buf,
                    ctx,
                )?;
                *self = Self::Status {
                    communication_token: String::new(),
                    statuses,
                };
                Ok(())
            }
            2 => {
                let mut error = if let Self::Error { error, .. } = self {
                    std::mem::take(error)
                } else {
                    String::new()
                };
                ProtoField::merge_field(wire_type, &mut error, buf, ctx)?;
                *self = Self::Error {
                    communication_token: String::new(),
                    error,
                };
                Ok(())
            }
            _ => unreachable!("only oneof member tags are routed here"),
        }
    }
}

impl prost::Message for Response {
    fn encode_raw(&self, buf: &mut impl prost::bytes::BufMut) {
        let token = match self {
            Self::Status {
                communication_token,
                statuses,
            } => {
                <VecWrapper<1> as ProtoAdapter<Vec<Status>>>::encode_field(1, statuses, buf);
                communication_token
            }
            Self::Error {
                communication_token,
                error,
            } => {
                // Oneof presence: the member is emitted even when empty.
                ProtoField::encode_field(2, error, buf);
                communication_token
            }
        };
        if !token.is_empty() {
            ProtoField::encode_field(4, token, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut impl prost::bytes::Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1..=2 => {
                let token = std::mem::take(self.token_mut());
                self.merge_variant(tag, wire_type, buf, ctx)?;
                *self.token_mut() = token;
                Ok(())
            }
            4 => {
                let token = self.token_mut();
                ProtoField::merge_field(wire_type, token, buf, ctx)
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    /// Top-level decode entry, order-tolerant for the sibling token. An
    /// absent oneof decodes to the default (empty `Error`), like the
    /// historical conversion — which also kept the token there.
    fn merge(&mut self, mut buf: impl prost::bytes::Buf) -> Result<(), prost::DecodeError>
    where
        Self: Sized,
    {
        let ctx = prost::encoding::DecodeContext::default();
        let mut token = std::mem::take(self.token_mut());
        while buf.has_remaining() {
            let (tag, wire_type) = prost::encoding::decode_key(&mut buf)?;
            match tag {
                1..=2 => self.merge_variant(tag, wire_type, &mut buf, ctx.clone())?,
                4 => {
                    ProtoField::merge_field(wire_type, &mut token, &mut buf, ctx.clone())?;
                }
                _ => prost::encoding::skip_field(wire_type, tag, &mut buf, ctx.clone())?,
            }
        }
        *self.token_mut() = token;
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        let (payload_len, token) = match self {
            Self::Status {
                communication_token,
                statuses,
            } => (
                <VecWrapper<1> as ProtoAdapter<Vec<Status>>>::encoded_len_field(1, statuses),
                communication_token,
            ),
            Self::Error {
                communication_token,
                error,
            } => (ProtoField::encoded_len_field(2, error), communication_token),
        };
        let token_len = if token.is_empty() {
            0
        } else {
            ProtoField::encoded_len_field(4, token)
        };
        payload_len + token_len
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

impl From<Response> for v3::agent::CreateTaskReply {
    fn from(value: Response) -> Self {
        match value {
            Response::Status {
                communication_token,
                statuses,
            } => Self {
                communication_token,
                response: Some(v3::agent::create_task_reply::Response::CreationStatusList(
                    v3::agent::create_task_reply::CreationStatusList {
                        creation_statuses: statuses.into_collect(),
                    },
                )),
            },
            Response::Error {
                communication_token,
                error,
            } => Self {
                communication_token,
                response: Some(v3::agent::create_task_reply::Response::Error(error)),
            },
        }
    }
}

impl From<v3::agent::CreateTaskReply> for Response {
    fn from(value: v3::agent::CreateTaskReply) -> Self {
        match value.response {
            Some(v3::agent::create_task_reply::Response::CreationStatusList(status)) => {
                Self::Status {
                    communication_token: value.communication_token,
                    statuses: status.creation_statuses.into_collect(),
                }
            }
            Some(v3::agent::create_task_reply::Response::Error(error)) => Self::Error {
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

super::super::impl_convert!(req Response : v3::agent::CreateTaskReply);

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::{Request, Response, Status};
    use crate::api::v3;
    use crate::objects::{DataChunk, InitTaskRequest, TaskRequestHeader};

    fn v3_request_samples() -> Vec<v3::agent::CreateTaskRequest> {
        vec![
            v3::agent::CreateTaskRequest {
                communication_token: "token-1".into(),
                r#type: Some(v3::agent::create_task_request::Type::InitRequest(
                    v3::agent::create_task_request::InitRequest {
                        task_options: Some(v3::TaskOptions {
                            // Explicit so the round-trip is the identity: an
                            // absent duration folds to INFINITE_DURATION.
                            max_duration: Some(prost_types::Duration {
                                seconds: 60,
                                nanos: 0,
                            }),
                            max_retries: 3,
                            partition_id: "part".into(),
                            ..Default::default()
                        }),
                    },
                )),
            },
            v3::agent::CreateTaskRequest {
                communication_token: "token-2".into(),
                r#type: Some(v3::agent::create_task_request::Type::InitTask(
                    v3::InitTaskRequest {
                        r#type: Some(v3::init_task_request::Type::Header(v3::TaskRequestHeader {
                            expected_output_keys: vec!["out".into()],
                            data_dependencies: vec!["dep".into()],
                        })),
                    },
                )),
            },
            v3::agent::CreateTaskRequest {
                communication_token: "token-3".into(),
                r#type: Some(v3::agent::create_task_request::Type::TaskPayload(
                    v3::DataChunk {
                        r#type: Some(v3::data_chunk::Type::Data(b"chunk".to_vec())),
                    },
                )),
            },
        ]
    }

    #[test]
    fn request_roundtrips_through_generated_type() {
        for theirs in v3_request_samples() {
            let ours = Request::decode(theirs.encode_to_vec().as_slice()).unwrap();
            let back =
                v3::agent::CreateTaskRequest::decode(ours.encode_to_vec().as_slice()).unwrap();
            assert_eq!(back, theirs);
        }
    }

    #[test]
    fn request_token_before_member_is_kept() {
        let mut buf = Vec::new();
        prost::encoding::string::encode(4, &"early".to_owned(), &mut buf);
        crate::codec::message::encode(
            2,
            &InitTaskRequest::Header(TaskRequestHeader::default()),
            &mut buf,
        );
        let ours = Request::decode(buf.as_slice()).unwrap();
        let Request::InitTaskRequest {
            communication_token,
            ..
        } = &ours
        else {
            panic!("expected InitTaskRequest, got {ours:?}");
        };
        assert_eq!(communication_token, "early");
    }

    #[test]
    fn request_token_without_member_decodes_as_invalid() {
        let mut buf = Vec::new();
        prost::encoding::string::encode(4, &"lonely".to_owned(), &mut buf);
        let ours = Request::decode(buf.as_slice()).unwrap();
        assert!(matches!(ours, Request::Invalid));
    }

    #[test]
    fn response_roundtrips_through_generated_type() {
        let ours = Response::Status {
            communication_token: "token".into(),
            statuses: vec![
                Status::TaskInfo {
                    task_id: "task".into(),
                    expected_output_keys: vec!["out".into()],
                    data_dependencies: vec![],
                    payload_id: "payload".into(),
                },
                Status::Error("boom".into()),
            ],
        };
        let theirs = v3::agent::CreateTaskReply::decode(ours.encode_to_vec().as_slice()).unwrap();
        assert_eq!(theirs, v3::agent::CreateTaskReply::from(ours.clone()));

        let back = Response::decode(theirs.encode_to_vec().as_slice()).unwrap();
        let Response::Status { statuses, .. } = back else {
            panic!("expected Status");
        };
        assert_eq!(statuses.len(), 2);
    }

    #[test]
    fn data_chunk_variant_keeps_bytes() {
        let ours = Request::DataChunk {
            communication_token: "t".into(),
            chunk: DataChunk::Data(bytes::Bytes::from_static(b"payload")),
        };
        let theirs = v3::agent::CreateTaskRequest::decode(ours.encode_to_vec().as_slice()).unwrap();
        assert_eq!(
            theirs.r#type,
            Some(v3::agent::create_task_request::Type::TaskPayload(
                v3::DataChunk {
                    r#type: Some(v3::data_chunk::Type::Data(b"payload".to_vec())),
                }
            ))
        );
    }
}
