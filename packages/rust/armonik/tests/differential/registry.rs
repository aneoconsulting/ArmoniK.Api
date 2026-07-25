//! Mapping from proto full names to the armonik types implementing them.
//!
//! Grown in lockstep with the annotation of `src/objects/`: registering a
//! type here removes it from `TEMP_UNMAPPED` in `main.rs` (the coverage
//! test enforces both directions).

use armonik::reexports::prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage, Value};

pub struct Entry {
    pub proto: &'static str,
    /// Decode the bytes as the armonik type and re-encode them.
    pub roundtrip: fn(&[u8]) -> Result<Vec<u8>, armonik::reexports::prost::DecodeError>,
    /// Projection onto the type's documented equivalence classes, applied to
    /// both sides before the semantic comparison. Needed only where an
    /// armonik type has a custom default that differs from the proto zero
    /// value (see `normalize_task_options`).
    pub normalize: Option<fn(&mut DynamicMessage)>,
}

macro_rules! registry {
    ($($proto:literal => $ty:ty $(, normalize = $normalize:expr)?),* $(,)?) => {
        pub fn entries() -> Vec<Entry> {
            vec![$(Entry {
                proto: $proto,
                roundtrip: |bytes| Ok(<$ty as Message>::decode(bytes)?.encode_to_vec()),
                #[allow(unused_mut, unused_assignments)]
                normalize: {
                    let mut normalize: Option<fn(&mut DynamicMessage)> = None;
                    $(normalize = Some($normalize);)?
                    normalize
                },
            }),*]
        }
    };
}

registry! {
    "armonik.api.grpc.v1.DataChunk" => armonik::DataChunk,
        normalize = |message| {
            normalize_bool_marker(message, "data_complete");
            normalize_default_member(message, "data");
        },
    "armonik.api.grpc.v1.InitKeyedDataStream" => armonik::InitKeyedDataStream,
        normalize = |message| {
            normalize_bool_marker(message, "last_result");
            normalize_default_member(message, "key");
        },
    "armonik.api.grpc.v1.InitTaskRequest" => armonik::InitTaskRequest,
        normalize = |message| {
            normalize_bool_marker(message, "last_task");
            normalize_default_member(message, "header");
        },
    "armonik.api.grpc.v1.Output" => armonik::Output,
        normalize = |message| normalize_default_member(message, "ok"),
    "armonik.api.grpc.v1.Session" => armonik::Session,
    "armonik.api.grpc.v1.StatusCount" => armonik::StatusCount,
    "armonik.api.grpc.v1.TaskOptions" => armonik::TaskOptions,
        normalize = normalize_task_options,
    "armonik.api.grpc.v1.TaskRequestHeader" => armonik::TaskRequestHeader,
}

/// Marker oneof members: the armonik types only remember *which* member was
/// set, so a pathological explicit `false` re-encodes as `true` — exactly
/// like the historical conversions. Project the member onto `true`.
fn normalize_bool_marker(message: &mut DynamicMessage, member: &str) {
    let field = message
        .descriptor()
        .get_field_by_name(member)
        .expect("marker member exists");
    if message.has_field(&field) {
        message.set_field(&field, Value::Bool(true));
    }
}

/// Flattened oneofs whose Rust `Default` is a member variant (`DataChunk` =
/// empty `Data`, `InitTaskRequest` = empty `Header`, ...): an absent oneof
/// decodes to that variant and re-encodes with the member present, exactly
/// like the historical `None => Default::default()` conversions. Project the
/// absent case onto the default member.
fn normalize_default_member(message: &mut DynamicMessage, member: &str) {
    let descriptor = message.descriptor();
    let oneof = descriptor.oneofs().next().expect("flattened oneof exists");
    let member_set = oneof.fields().any(|field| message.has_field(&field));
    if !member_set {
        let field = descriptor
            .get_field_by_name(member)
            .expect("default member exists");
        message.set_field(&field, Value::default_value_for_field(&field));
    }
}

/// `TaskOptions::default()` uses `INFINITE_DURATION` for `max_duration`, and
/// the wire mapping folds "absent" and "present but empty" (both encode zero
/// bytes) into that default. Project both representations onto the explicit
/// infinite duration so the comparison reflects the documented semantics.
fn normalize_task_options(message: &mut DynamicMessage) {
    let field = message
        .descriptor()
        .get_field_by_name("max_duration")
        .expect("TaskOptions.max_duration exists");
    let is_empty = match message.get_field(&field).as_ref() {
        Value::Message(duration) => duration.encode_to_vec().is_empty(),
        _ => true,
    };
    if is_empty {
        let duration_desc = match field.kind() {
            prost_reflect::Kind::Message(desc) => desc,
            other => panic!("max_duration should be a message, got {other:?}"),
        };
        let mut infinite = DynamicMessage::new(duration_desc);
        infinite.set_field_by_name("seconds", Value::I64(315576000000));
        message.set_field(&field, Value::Message(infinite));
    }
}
