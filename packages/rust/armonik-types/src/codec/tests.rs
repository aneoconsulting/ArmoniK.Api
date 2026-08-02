//! Differential tests of the codec building blocks against prost-generated
//! ground truth.
//!
//! The hand-written [`prost::Message`] implementations in this module are
//! also the prototypes of the code the `armonik-macros` derives emit: any
//! change to the emitted shape should be reflected here first.

use std::collections::HashMap;

use ::bytes::Bytes;
use prost::bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::Message;

use super::{enumeration, FieldKind, ProtoField};

/// Prototype of the `derive(armonik::Enum)` output (without the `Other`
/// catch-all, which is irrelevant to the wire format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TestEnum {
    #[default]
    Zero,
    One,
    Two,
}

impl From<i32> for TestEnum {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::One,
            2 => Self::Two,
            _ => Self::Zero,
        }
    }
}

impl From<TestEnum> for i32 {
    fn from(value: TestEnum) -> Self {
        value as i32
    }
}

impl ProtoField for TestEnum {
    const KIND: FieldKind = FieldKind::Enum;

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        enumeration::encode(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        enumeration::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        enumeration::encoded_len(tag, value)
    }

    fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        enumeration::encode_repeated(tag, values, buf);
    }

    fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
        enumeration::encoded_len_repeated(tag, values)
    }

    fn merge_repeated(
        wire_type: WireType,
        values: &mut Vec<Self>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        enumeration::merge_repeated(wire_type, values, buf, ctx)
    }
}

/// Prototype of the `derive(armonik::Message)` output for a plain struct,
/// mirroring `armonik.api.grpc.v1.TaskOptions` (map, non-`Option` message
/// with absent-as-default semantics, scalars).
#[derive(Debug, Clone, PartialEq, Default)]
struct TestOptions {
    options: HashMap<String, String>,
    max_duration: prost_types::Duration,
    max_retries: i32,
    priority: i32,
    partition_id: String,
    application_name: String,
    application_version: String,
    application_namespace: String,
    application_service: String,
    engine_type: String,
}

impl Message for TestOptions {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !ProtoField::is_default(&self.options) {
            ProtoField::encode_field(1, &self.options, buf);
        }
        if !ProtoField::is_default(&self.max_duration) {
            ProtoField::encode_field(2, &self.max_duration, buf);
        }
        if !ProtoField::is_default(&self.max_retries) {
            ProtoField::encode_field(3, &self.max_retries, buf);
        }
        if !ProtoField::is_default(&self.priority) {
            ProtoField::encode_field(4, &self.priority, buf);
        }
        if !ProtoField::is_default(&self.partition_id) {
            ProtoField::encode_field(5, &self.partition_id, buf);
        }
        if !ProtoField::is_default(&self.application_name) {
            ProtoField::encode_field(6, &self.application_name, buf);
        }
        if !ProtoField::is_default(&self.application_version) {
            ProtoField::encode_field(7, &self.application_version, buf);
        }
        if !ProtoField::is_default(&self.application_namespace) {
            ProtoField::encode_field(8, &self.application_namespace, buf);
        }
        if !ProtoField::is_default(&self.application_service) {
            ProtoField::encode_field(9, &self.application_service, buf);
        }
        if !ProtoField::is_default(&self.engine_type) {
            ProtoField::encode_field(10, &self.engine_type, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1 => ProtoField::merge_field(wire_type, &mut self.options, buf, ctx),
            2 => ProtoField::merge_field(wire_type, &mut self.max_duration, buf, ctx),
            3 => ProtoField::merge_field(wire_type, &mut self.max_retries, buf, ctx),
            4 => ProtoField::merge_field(wire_type, &mut self.priority, buf, ctx),
            5 => ProtoField::merge_field(wire_type, &mut self.partition_id, buf, ctx),
            6 => ProtoField::merge_field(wire_type, &mut self.application_name, buf, ctx),
            7 => ProtoField::merge_field(wire_type, &mut self.application_version, buf, ctx),
            8 => ProtoField::merge_field(wire_type, &mut self.application_namespace, buf, ctx),
            9 => ProtoField::merge_field(wire_type, &mut self.application_service, buf, ctx),
            10 => ProtoField::merge_field(wire_type, &mut self.engine_type, buf, ctx),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !ProtoField::is_default(&self.options) {
            len += ProtoField::encoded_len_field(1, &self.options);
        }
        if !ProtoField::is_default(&self.max_duration) {
            len += ProtoField::encoded_len_field(2, &self.max_duration);
        }
        if !ProtoField::is_default(&self.max_retries) {
            len += ProtoField::encoded_len_field(3, &self.max_retries);
        }
        if !ProtoField::is_default(&self.priority) {
            len += ProtoField::encoded_len_field(4, &self.priority);
        }
        if !ProtoField::is_default(&self.partition_id) {
            len += ProtoField::encoded_len_field(5, &self.partition_id);
        }
        if !ProtoField::is_default(&self.application_name) {
            len += ProtoField::encoded_len_field(6, &self.application_name);
        }
        if !ProtoField::is_default(&self.application_version) {
            len += ProtoField::encoded_len_field(7, &self.application_version);
        }
        if !ProtoField::is_default(&self.application_namespace) {
            len += ProtoField::encoded_len_field(8, &self.application_namespace);
        }
        if !ProtoField::is_default(&self.application_service) {
            len += ProtoField::encoded_len_field(9, &self.application_service);
        }
        if !ProtoField::is_default(&self.engine_type) {
            len += ProtoField::encoded_len_field(10, &self.engine_type);
        }
        len
    }

    fn clear(&mut self) {
        *self = ::core::default::Default::default();
    }
}

/// prost-derived ground truth covering every container/presence shape.
#[derive(Clone, PartialEq, Message)]
struct RefShapes {
    #[prost(int32, repeated, tag = "1")]
    numbers: Vec<i32>,
    #[prost(string, optional, tag = "2")]
    name: Option<String>,
    #[prost(bytes = "vec", tag = "3")]
    blob: Vec<u8>,
    #[prost(string, repeated, tag = "4")]
    names: Vec<String>,
    #[prost(message, repeated, tag = "5")]
    durations: Vec<prost_types::Duration>,
    #[prost(int32, repeated, tag = "6")]
    enums: Vec<i32>,
    #[prost(double, tag = "7")]
    real: f64,
    #[prost(uint64, tag = "8")]
    big: u64,
    #[prost(bool, tag = "9")]
    flag: bool,
}

/// Our mirror of [`RefShapes`], in the derive-emitted shape.
#[derive(Debug, Clone, PartialEq, Default)]
struct OurShapes {
    numbers: Vec<i32>,
    name: Option<String>,
    blob: Bytes,
    names: Vec<String>,
    durations: Vec<prost_types::Duration>,
    enums: Vec<TestEnum>,
    real: f64,
    big: u64,
    flag: bool,
}

impl Message for OurShapes {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !ProtoField::is_default(&self.numbers) {
            ProtoField::encode_field(1, &self.numbers, buf);
        }
        if !ProtoField::is_default(&self.name) {
            ProtoField::encode_field(2, &self.name, buf);
        }
        if !ProtoField::is_default(&self.blob) {
            ProtoField::encode_field(3, &self.blob, buf);
        }
        if !ProtoField::is_default(&self.names) {
            ProtoField::encode_field(4, &self.names, buf);
        }
        if !ProtoField::is_default(&self.durations) {
            ProtoField::encode_field(5, &self.durations, buf);
        }
        if !ProtoField::is_default(&self.enums) {
            ProtoField::encode_field(6, &self.enums, buf);
        }
        if !ProtoField::is_default(&self.real) {
            ProtoField::encode_field(7, &self.real, buf);
        }
        if !ProtoField::is_default(&self.big) {
            ProtoField::encode_field(8, &self.big, buf);
        }
        if !ProtoField::is_default(&self.flag) {
            ProtoField::encode_field(9, &self.flag, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1 => ProtoField::merge_field(wire_type, &mut self.numbers, buf, ctx),
            2 => ProtoField::merge_field(wire_type, &mut self.name, buf, ctx),
            3 => ProtoField::merge_field(wire_type, &mut self.blob, buf, ctx),
            4 => ProtoField::merge_field(wire_type, &mut self.names, buf, ctx),
            5 => ProtoField::merge_field(wire_type, &mut self.durations, buf, ctx),
            6 => ProtoField::merge_field(wire_type, &mut self.enums, buf, ctx),
            7 => ProtoField::merge_field(wire_type, &mut self.real, buf, ctx),
            8 => ProtoField::merge_field(wire_type, &mut self.big, buf, ctx),
            9 => ProtoField::merge_field(wire_type, &mut self.flag, buf, ctx),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !ProtoField::is_default(&self.numbers) {
            len += ProtoField::encoded_len_field(1, &self.numbers);
        }
        if !ProtoField::is_default(&self.name) {
            len += ProtoField::encoded_len_field(2, &self.name);
        }
        if !ProtoField::is_default(&self.blob) {
            len += ProtoField::encoded_len_field(3, &self.blob);
        }
        if !ProtoField::is_default(&self.names) {
            len += ProtoField::encoded_len_field(4, &self.names);
        }
        if !ProtoField::is_default(&self.durations) {
            len += ProtoField::encoded_len_field(5, &self.durations);
        }
        if !ProtoField::is_default(&self.enums) {
            len += ProtoField::encoded_len_field(6, &self.enums);
        }
        if !ProtoField::is_default(&self.real) {
            len += ProtoField::encoded_len_field(7, &self.real);
        }
        if !ProtoField::is_default(&self.big) {
            len += ProtoField::encoded_len_field(8, &self.big);
        }
        if !ProtoField::is_default(&self.flag) {
            len += ProtoField::encoded_len_field(9, &self.flag);
        }
        len
    }

    fn clear(&mut self) {
        *self = ::core::default::Default::default();
    }
}

fn sample_options() -> TestOptions {
    TestOptions {
        options: [("key1", "value1"), ("key2", "value2")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        max_duration: prost_types::Duration {
            seconds: 300,
            nanos: 42,
        },
        max_retries: 5,
        priority: -3,
        partition_id: "partition".into(),
        application_name: "app".into(),
        application_version: "1.2.3".into(),
        application_namespace: "ns".into(),
        application_service: "svc".into(),
        engine_type: "engine".into(),
    }
}

/// prost-derived reference for `armonik.api.grpc.v1.TaskOptions` (extern'd,
/// so no generated type exists anymore).
#[derive(Clone, PartialEq, Message)]
struct RefOptions {
    #[prost(map = "string, string", tag = "1")]
    options: HashMap<String, String>,
    #[prost(message, optional, tag = "2")]
    max_duration: Option<prost_types::Duration>,
    #[prost(int32, tag = "3")]
    max_retries: i32,
    #[prost(int32, tag = "4")]
    priority: i32,
    #[prost(string, tag = "5")]
    partition_id: String,
    #[prost(string, tag = "6")]
    application_name: String,
    #[prost(string, tag = "7")]
    application_version: String,
    #[prost(string, tag = "8")]
    application_namespace: String,
    #[prost(string, tag = "9")]
    application_service: String,
    #[prost(string, tag = "10")]
    engine_type: String,
}

#[test]
fn options_roundtrip_through_generated_type() {
    let ours = sample_options();
    let theirs = RefOptions::decode(ours.encode_to_vec().as_slice()).unwrap();

    assert_eq!(theirs.options, ours.options);
    assert_eq!(theirs.max_duration, Some(ours.max_duration));
    assert_eq!(theirs.max_retries, ours.max_retries);
    assert_eq!(theirs.priority, ours.priority);
    assert_eq!(theirs.partition_id, ours.partition_id);
    assert_eq!(theirs.engine_type, ours.engine_type);

    let back = TestOptions::decode(theirs.encode_to_vec().as_slice()).unwrap();
    assert_eq!(back, ours);
}

#[test]
fn absent_message_field_decodes_as_default() {
    let theirs = RefOptions {
        max_duration: None,
        max_retries: 7,
        ..Default::default()
    };
    let ours = TestOptions::decode(theirs.encode_to_vec().as_slice()).unwrap();
    assert_eq!(ours.max_duration, prost_types::Duration::default());
    assert_eq!(ours.max_retries, 7);
}

#[test]
fn default_message_field_is_omitted_on_encode() {
    let ours = TestOptions {
        max_retries: 7,
        ..Default::default()
    };
    let theirs = RefOptions::decode(ours.encode_to_vec().as_slice()).unwrap();
    // Semantically indistinguishable for absent-as-default fields.
    assert_eq!(theirs.max_duration, None);
}

#[test]
fn present_default_message_field_decodes_as_default() {
    let theirs = RefOptions {
        max_duration: Some(prost_types::Duration::default()),
        ..Default::default()
    };
    let ours = TestOptions::decode(theirs.encode_to_vec().as_slice()).unwrap();
    assert_eq!(ours.max_duration, prost_types::Duration::default());
}

fn sample_shapes() -> OurShapes {
    OurShapes {
        numbers: vec![0, 1, -1, i32::MAX, i32::MIN, 300],
        name: Some(String::new()),
        blob: Bytes::from_static(b"\x00\x01\x02payload"),
        names: vec![String::new(), "second".to_owned()],
        durations: vec![
            prost_types::Duration::default(),
            prost_types::Duration {
                seconds: -5,
                nanos: -500,
            },
        ],
        enums: vec![TestEnum::Zero, TestEnum::Two, TestEnum::One],
        real: -2.5,
        big: u64::MAX,
        flag: true,
    }
}

#[test]
fn shapes_roundtrip_through_prost_derive() {
    let ours = sample_shapes();
    let theirs = RefShapes::decode(ours.encode_to_vec().as_slice()).unwrap();

    assert_eq!(theirs.numbers, ours.numbers);
    assert_eq!(theirs.name, ours.name);
    assert_eq!(theirs.blob, ours.blob);
    assert_eq!(theirs.names, ours.names);
    assert_eq!(theirs.durations, ours.durations);
    assert_eq!(
        theirs.enums,
        ours.enums.iter().map(|e| i32::from(*e)).collect::<Vec<_>>()
    );
    assert_eq!(theirs.real, ours.real);
    assert_eq!(theirs.big, ours.big);
    assert_eq!(theirs.flag, ours.flag);

    let back = OurShapes::decode(theirs.encode_to_vec().as_slice()).unwrap();
    assert_eq!(back, ours);

    // Without maps (whose iteration order is unstable), the encoding should
    // be byte-identical to prost's.
    assert_eq!(ours.encode_to_vec(), theirs.encode_to_vec());
}

#[test]
fn optional_presence_is_exact() {
    // `Some("")` must stay distinguishable from `None`.
    let ours = OurShapes {
        name: Some(String::new()),
        ..Default::default()
    };
    let theirs = RefShapes::decode(ours.encode_to_vec().as_slice()).unwrap();
    assert_eq!(theirs.name, Some(String::new()));

    let ours = OurShapes::default();
    let theirs = RefShapes::decode(ours.encode_to_vec().as_slice()).unwrap();
    assert_eq!(theirs.name, None);
}

#[test]
fn unpacked_repeated_scalars_are_accepted() {
    // Conforming proto3 writers pack numeric repeated fields, but decoders
    // must accept the unpacked form too.
    let mut buf = Vec::new();
    for value in [1i32, -1, 300] {
        prost::encoding::int32::encode(1, &value, &mut buf);
    }
    for value in [2i32, 1] {
        prost::encoding::int32::encode(6, &value, &mut buf);
    }
    let ours = OurShapes::decode(buf.as_slice()).unwrap();
    assert_eq!(ours.numbers, vec![1, -1, 300]);
    assert_eq!(ours.enums, vec![TestEnum::Two, TestEnum::One]);
}

#[test]
fn message_clear_resets_to_default() {
    let mut ours = sample_shapes();
    ours.clear();
    assert_eq!(ours, OurShapes::default());
}

// ---------------------------------------------------------------------------
// Prototypes of the two hard shapes: a whole-message oneof flattened into an
// enum (template for the `oneof` derive mode), and a oneof with a sibling
// field flattened into enum variants (template for the hand-written
// `agent::create_tasks` types). Ground truth: the real generated agent types.
// ---------------------------------------------------------------------------

use ::prost::encoding::message as message_codec;
use super::ProtoOneof;

/// Mirror of `armonik.api.grpc.v1.DataChunk`: whole-message oneof
/// { bytes data = 1; bool data_complete = 2; }, default variant `Data("")`.
#[derive(Debug, Clone, PartialEq)]
enum TestDataChunk {
    Data(Bytes),
    Complete,
}

impl Default for TestDataChunk {
    fn default() -> Self {
        Self::Data(Bytes::new())
    }
}

impl ProtoOneof for TestDataChunk {
    fn encode_oneof(value: &Self, buf: &mut impl BufMut) {
        // Oneof presence is significant: the active field is always emitted,
        // even when its payload is the default.
        match value {
            Self::Data(data) => ProtoField::encode_field(1, data, buf),
            Self::Complete => ProtoField::encode_field(2, &true, buf),
        }
    }

    fn merge_oneof(
        tag: u32,
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1 => {
                let mut data = if let Self::Data(data) = value {
                    std::mem::take(data)
                } else {
                    Bytes::new()
                };
                ProtoField::merge_field(wire_type, &mut data, buf, ctx)?;
                *value = Self::Data(data);
                Ok(())
            }
            2 => {
                let mut marker = false;
                ProtoField::merge_field(wire_type, &mut marker, buf, ctx)?;
                *value = Self::Complete;
                Ok(())
            }
            _ => unreachable!("oneof tags are routed by the containing message"),
        }
    }

    fn encoded_len_oneof(value: &Self) -> usize {
        match value {
            Self::Data(data) => ProtoField::encoded_len_field(1, data),
            Self::Complete => ProtoField::encoded_len_field(2, &true),
        }
    }
}

impl Message for TestDataChunk {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        ProtoOneof::encode_oneof(self, buf);
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1..=2 => ProtoOneof::merge_oneof(tag, wire_type, self, buf, ctx),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        ProtoOneof::encoded_len_oneof(self)
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Mirror of `armonik.api.grpc.v1.InitTaskRequest`: whole-message oneof
/// { TaskRequestHeader header = 1; bool last_task = 2; } with a message
/// payload and a marker variant.
#[derive(Debug, Clone, PartialEq, Default)]
enum TestInitTask {
    #[default]
    Invalid,
    Header(TestHeader),
    LastTask,
}

/// Mirror of `armonik.api.grpc.v1.TaskRequestHeader`.
#[derive(Debug, Clone, PartialEq, Default)]
struct TestHeader {
    expected_output_keys: Vec<String>,
    data_dependencies: Vec<String>,
}

impl Message for TestHeader {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !ProtoField::is_default(&self.expected_output_keys) {
            ProtoField::encode_field(1, &self.expected_output_keys, buf);
        }
        if !ProtoField::is_default(&self.data_dependencies) {
            ProtoField::encode_field(2, &self.data_dependencies, buf);
        }
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1 => ProtoField::merge_field(wire_type, &mut self.expected_output_keys, buf, ctx),
            2 => ProtoField::merge_field(wire_type, &mut self.data_dependencies, buf, ctx),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        if !ProtoField::is_default(&self.expected_output_keys) {
            len += ProtoField::encoded_len_field(1, &self.expected_output_keys);
        }
        if !ProtoField::is_default(&self.data_dependencies) {
            len += ProtoField::encoded_len_field(2, &self.data_dependencies);
        }
        len
    }

    fn clear(&mut self) {
        *self = ::core::default::Default::default();
    }
}

impl ProtoOneof for TestInitTask {
    fn encode_oneof(value: &Self, buf: &mut impl BufMut) {
        match value {
            Self::Invalid => {}
            Self::Header(header) => message_codec::encode(1, header, buf),
            Self::LastTask => ProtoField::encode_field(2, &true, buf),
        }
    }

    fn merge_oneof(
        tag: u32,
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1 => {
                // Same-variant occurrences merge into the payload, like prost.
                let mut header = if let Self::Header(header) = value {
                    std::mem::take(header)
                } else {
                    TestHeader::default()
                };
                message_codec::merge(wire_type, &mut header, buf, ctx)?;
                *value = Self::Header(header);
                Ok(())
            }
            2 => {
                let mut marker = false;
                ProtoField::merge_field(wire_type, &mut marker, buf, ctx)?;
                *value = Self::LastTask;
                Ok(())
            }
            _ => unreachable!("oneof tags are routed by the containing message"),
        }
    }

    fn encoded_len_oneof(value: &Self) -> usize {
        match value {
            Self::Invalid => 0,
            Self::Header(header) => message_codec::encoded_len(1, header),
            Self::LastTask => ProtoField::encoded_len_field(2, &true),
        }
    }
}

impl Message for TestInitTask {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        ProtoOneof::encode_oneof(self, buf);
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        match tag {
            1..=2 => ProtoOneof::merge_oneof(tag, wire_type, self, buf, ctx),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        ProtoOneof::encoded_len_oneof(self)
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

/// The derives also emit a [`ProtoField`] impl for every message type so it
/// composes as a field of other messages; this is its template.
impl ProtoField for TestOptions {
    const KIND: FieldKind = FieldKind::Message;
    const NAMES: &'static [&'static str] = &["armonik.api.grpc.v1.TaskOptions"];

    fn encode_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        message_codec::encode(tag, value, buf);
    }

    fn merge_field(
        wire_type: WireType,
        value: &mut Self,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        message_codec::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_field(tag: u32, value: &Self) -> usize {
        message_codec::encoded_len(tag, value)
    }

    fn is_default(value: &Self) -> bool {
        super::message_is_default(value)
    }

    fn encode_repeated(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        message_codec::encode_repeated(tag, values, buf);
    }

    fn encoded_len_repeated(tag: u32, values: &[Self]) -> usize {
        message_codec::encoded_len_repeated(tag, values)
    }

    fn merge_repeated(
        wire_type: WireType,
        values: &mut Vec<Self>,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        message_codec::merge_repeated(wire_type, values, buf, ctx)
    }
}

#[test]
fn whole_message_oneof_roundtrips() {
    let cases = [
        (
            TestInitTask::Header(TestHeader {
                expected_output_keys: vec!["a".into()],
                data_dependencies: vec![],
            }),
            crate::InitTaskRequest::Header(crate::TaskRequestHeader {
                expected_output_keys: vec!["a".into()],
                data_dependencies: vec![],
            }),
        ),
        (TestInitTask::LastTask, crate::InitTaskRequest::LastTask),
    ];
    for (ours, theirs) in cases {
        assert_eq!(ours.encode_to_vec(), theirs.encode_to_vec());
        assert_eq!(
            TestInitTask::decode(theirs.encode_to_vec().as_slice()).unwrap(),
            ours
        );
    }
    // "No member set" encodes to nothing and decodes to the default variant.
    assert!(TestInitTask::Invalid.encode_to_vec().is_empty());
    assert_eq!(
        TestInitTask::decode(&[][..]).unwrap(),
        TestInitTask::Invalid
    );

    // Default-payload oneof fields are still emitted (oneof presence).
    let ours = TestDataChunk::Data(Bytes::new());
    let theirs = crate::DataChunk::Data(Bytes::new());
    assert_eq!(ours.encode_to_vec(), theirs.encode_to_vec());
    assert!(!ours.encode_to_vec().is_empty());
}
