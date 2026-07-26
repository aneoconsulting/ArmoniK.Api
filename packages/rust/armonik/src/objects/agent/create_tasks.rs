use super::super::{DataChunk, InitTaskRequest, TaskOptions};
use crate::codec::{adapters::VecWrapper, message as message_codec, ProtoAdapter, ProtoField};

#[derive(Debug, Clone, Default, armonik_macros::Message)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateTaskRequest.InitRequest")]
pub struct InitRequest {
    pub task_options: Option<TaskOptions>,
}

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

// Hand-written rather than derived. Everything the derive emits (encode
// fragments, merge arms, decode seeds, descriptor asserts) is generated from
// a one-Rust-field-to-one-proto-field correspondence; `with` adapters can
// change how a single field is encoded, but not that arity. Here one Rust
// enum stands for the oneof AND its sibling token: every variant carries the
// token, so both wire fields feed every variant, and merging needs
// cross-field state (the token may precede the member on the wire). Teaching
// the derive this shape would cost more grammar and codegen than the four
// hand-written impls it would replace (this one, [`Response`],
// `tasks::Output`, `agent::notify_result_data::Request`), each used exactly
// once. The differential harness fuzzes them against `DynamicMessage` ground
// truth exactly like the derived types.
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

#[derive(Debug, Clone, PartialEq, Eq, armonik_macros::Message)]
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

// Hand-written for the same reason as [`Request`]: one Rust enum stands for
// the oneof and its sibling token, which breaks the one-field-to-one-field
// correspondence the derive is built on (see the comment there).
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

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::{Request, Response, Status};
    use crate::objects::{DataChunk, InitTaskRequest, TaskRequestHeader};

    // prost-derived ground truth, mirroring the proto definitions (the
    // generated types no longer exist for these extern'd messages).

    #[derive(Clone, PartialEq, Message)]
    struct RefRequest {
        #[prost(oneof = "RefRequestType", tags = "1, 2, 3")]
        r#type: Option<RefRequestType>,
        #[prost(string, tag = "4")]
        communication_token: String,
    }

    #[derive(Clone, PartialEq, prost::Oneof)]
    enum RefRequestType {
        #[prost(message, tag = "1")]
        InitRequest(RefInitRequest),
        #[prost(message, tag = "2")]
        InitTask(crate::InitTaskRequest),
        #[prost(message, tag = "3")]
        TaskPayload(crate::DataChunk),
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefInitRequest {
        #[prost(message, optional, tag = "1")]
        task_options: Option<crate::TaskOptions>,
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefReply {
        #[prost(oneof = "RefReplyType", tags = "1, 2")]
        response: Option<RefReplyType>,
        #[prost(string, tag = "4")]
        communication_token: String,
    }

    #[derive(Clone, PartialEq, prost::Oneof)]
    enum RefReplyType {
        #[prost(message, tag = "1")]
        CreationStatusList(RefCreationStatusList),
        #[prost(string, tag = "2")]
        Error(String),
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefCreationStatusList {
        #[prost(message, repeated, tag = "1")]
        creation_statuses: Vec<RefCreationStatus>,
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefCreationStatus {
        #[prost(oneof = "RefStatusType", tags = "1, 2")]
        status: Option<RefStatusType>,
    }

    #[derive(Clone, PartialEq, prost::Oneof)]
    enum RefStatusType {
        #[prost(message, tag = "1")]
        TaskInfo(RefTaskInfo),
        #[prost(string, tag = "2")]
        Error(String),
    }

    #[derive(Clone, PartialEq, Message)]
    struct RefTaskInfo {
        #[prost(string, tag = "1")]
        task_id: String,
        #[prost(string, repeated, tag = "2")]
        expected_output_keys: Vec<String>,
        #[prost(string, repeated, tag = "3")]
        data_dependencies: Vec<String>,
        #[prost(string, tag = "4")]
        payload_id: String,
    }

    fn ref_request_samples() -> Vec<RefRequest> {
        vec![
            RefRequest {
                communication_token: "token-1".into(),
                r#type: Some(RefRequestType::InitRequest(RefInitRequest {
                    task_options: Some(crate::TaskOptions {
                        max_retries: 3,
                        partition_id: "part".into(),
                        ..Default::default()
                    }),
                })),
            },
            RefRequest {
                communication_token: "token-2".into(),
                r#type: Some(RefRequestType::InitTask(crate::InitTaskRequest::Header(
                    crate::TaskRequestHeader {
                        expected_output_keys: vec!["out".into()],
                        data_dependencies: vec!["dep".into()],
                    },
                ))),
            },
            RefRequest {
                communication_token: "token-3".into(),
                r#type: Some(RefRequestType::TaskPayload(crate::DataChunk::Data(
                    bytes::Bytes::from_static(b"chunk"),
                ))),
            },
        ]
    }

    #[test]
    fn request_roundtrips_through_reference_encoding() {
        for theirs in ref_request_samples() {
            let ours = Request::decode(theirs.encode_to_vec().as_slice()).unwrap();
            let back = RefRequest::decode(ours.encode_to_vec().as_slice()).unwrap();
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
    fn response_roundtrips_through_reference_encoding() {
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
        let theirs = RefReply::decode(ours.encode_to_vec().as_slice()).unwrap();
        assert_eq!(theirs.communication_token, "token");
        let Some(RefReplyType::CreationStatusList(list)) = &theirs.response else {
            panic!("expected CreationStatusList");
        };
        assert_eq!(
            list.creation_statuses,
            vec![
                RefCreationStatus {
                    status: Some(RefStatusType::TaskInfo(RefTaskInfo {
                        task_id: "task".into(),
                        expected_output_keys: vec!["out".into()],
                        data_dependencies: vec![],
                        payload_id: "payload".into(),
                    })),
                },
                RefCreationStatus {
                    status: Some(RefStatusType::Error("boom".into())),
                },
            ]
        );

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
        let theirs = RefRequest::decode(ours.encode_to_vec().as_slice()).unwrap();
        assert_eq!(
            theirs.r#type,
            Some(RefRequestType::TaskPayload(DataChunk::Data(
                bytes::Bytes::from_static(b"payload"),
            )))
        );
    }
}
