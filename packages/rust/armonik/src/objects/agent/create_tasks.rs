use super::super::{DataChunk, InitTaskRequest, TaskOptions};

#[armonik_macros::message]
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateTaskRequest.InitRequest")]
pub struct InitRequest {
    pub task_options: Option<TaskOptions>,
}

/// The `CreateTaskRequest` message: one oneof (tags 1-3) plus a sibling `communication_token = 4`,
/// carried by every variant, `Invalid` (the "no member set" case) included, so a token survives any
/// wire field order.
#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateTaskRequest")]
pub enum Request {
    Invalid {
        communication_token: String,
    },
    InitRequest {
        communication_token: String,
        request: InitRequest,
    },
    #[armonik(rename = "init_task")]
    InitTaskRequest {
        communication_token: String,
        request: InitTaskRequest,
    },
    #[armonik(rename = "task_payload")]
    DataChunk {
        communication_token: String,
        chunk: DataChunk,
    },
}

impl Default for Request {
    fn default() -> Self {
        Self::Invalid {
            communication_token: Default::default(),
        }
    }
}

#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateTaskReply.CreationStatus")]
pub enum Status {
    TaskInfo {
        task_id: String,
        expected_output_keys: Vec<String>,
        data_dependencies: Vec<String>,
        payload_id: String,
    },
    Error(String),
}

impl Default for Status {
    fn default() -> Self {
        Self::Error(Default::default())
    }
}

/// The `CreateTaskReply` message: one oneof (tags 1-2, with the `CreationStatusList` wrapper
/// flattened through `VecWrapper`) plus a sibling `communication_token = 4` carried by both
/// variants. There is no "no member set" variant: an absent oneof decodes to the `Error` default.
#[armonik_macros::message]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[armonik(message = "armonik.api.grpc.v1.agent.CreateTaskReply")]
pub enum Response {
    #[armonik(rename = "creation_status_list")]
    Status {
        communication_token: String,
        #[armonik(
            with = "crate::codec::adapters::Wrapper<1>",
            absorbs = "armonik.api.grpc.v1.agent.CreateTaskReply.CreationStatusList"
        )]
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

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::{Request, Response, Status};
    use crate::objects::{DataChunk, InitTaskRequest, TaskRequestHeader};

    // prost-derived ground truth, mirroring the proto definitions (the generated types no longer
    // exist for these extern'd messages).

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
        ::prost::encoding::message::encode(
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
    fn request_token_without_member_keeps_token() {
        // `Invalid` carries the sibling token like every variant, so a memberless message is
        // lossless (the historical conversion used to drop the token here).
        let mut buf = Vec::new();
        prost::encoding::string::encode(4, &"lonely".to_owned(), &mut buf);
        let ours = Request::decode(buf.as_slice()).unwrap();
        assert_eq!(
            ours,
            Request::Invalid {
                communication_token: "lonely".into(),
            }
        );
        assert_eq!(ours.encode_to_vec(), buf);
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
