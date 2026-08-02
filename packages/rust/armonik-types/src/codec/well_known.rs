//! The well-known types used by the API, gated into the blanket
//! message-kind [`ProtoField`](super::ProtoField) impl.

impl super::Msg for prost_types::Timestamp {
    const NAMES: &'static [&'static str] = &["google.protobuf.Timestamp"];
}

impl super::Msg for prost_types::Duration {
    const NAMES: &'static [&'static str] = &["google.protobuf.Duration"];
}
