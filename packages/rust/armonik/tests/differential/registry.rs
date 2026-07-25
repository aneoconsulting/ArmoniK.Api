//! Mapping from proto full names to the armonik types implementing them,
//! and the projection of messages onto the armonik types' documented
//! equivalence classes.
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
}

macro_rules! registry {
    ($($proto:literal => $ty:ty),* $(,)?) => {
        pub fn entries() -> Vec<Entry> {
            vec![$(Entry {
                proto: $proto,
                roundtrip: |bytes| Ok(<$ty as Message>::decode(bytes)?.encode_to_vec()),
            }),*]
        }
    };
}

registry! {
    "armonik.api.grpc.v1.Configuration" => armonik::Configuration,
    "armonik.api.grpc.v1.Count" => armonik::Count,
    "armonik.api.grpc.v1.Error" => armonik::Error,
    "armonik.api.grpc.v1.FilterArray" => armonik::FilterArray,
    "armonik.api.grpc.v1.FilterBoolean" => armonik::FilterBoolean,
    "armonik.api.grpc.v1.FilterDate" => armonik::FilterDate,
    "armonik.api.grpc.v1.FilterDuration" => armonik::FilterDuration,
    "armonik.api.grpc.v1.FilterNumber" => armonik::FilterNumber,
    "armonik.api.grpc.v1.FilterString" => armonik::FilterString,
    "armonik.api.grpc.v1.sessions.FilterStatus"
        => armonik::FilterStatus<armonik::SessionStatus>,
    "armonik.api.grpc.v1.tasks.FilterStatus"
        => armonik::FilterStatus<armonik::TaskStatus>,
    "armonik.api.grpc.v1.results.FilterStatus"
        => armonik::FilterStatus<armonik::ResultStatus>,
    "armonik.api.grpc.v1.ResultRequest" => armonik::ResultRequest,
    "armonik.api.grpc.v1.TaskError" => armonik::TaskError,
    "armonik.api.grpc.v1.TaskId" => armonik::TaskId,
    "armonik.api.grpc.v1.TaskIdList" => armonik::TaskIdList,
    "armonik.api.grpc.v1.TaskIdWithStatus" => armonik::TaskIdWithStatus,
    "armonik.api.grpc.v1.TaskList" => armonik::TaskList,
    "armonik.api.grpc.v1.TaskOutputRequest" => armonik::TaskOutputRequest,
    "armonik.api.grpc.v1.TaskRequest" => armonik::TaskRequest,
    "armonik.api.grpc.v1.DataChunk" => armonik::DataChunk,
    "armonik.api.grpc.v1.InitKeyedDataStream" => armonik::InitKeyedDataStream,
    "armonik.api.grpc.v1.InitTaskRequest" => armonik::InitTaskRequest,
    "armonik.api.grpc.v1.Output" => armonik::Output,
    "armonik.api.grpc.v1.Session" => armonik::Session,
    "armonik.api.grpc.v1.StatusCount" => armonik::StatusCount,
    "armonik.api.grpc.v1.TaskOptions" => armonik::TaskOptions,
    "armonik.api.grpc.v1.TaskRequestHeader" => armonik::TaskRequestHeader,
    "armonik.api.grpc.v1.agent.CreateTaskRequest" => armonik::agent::create_tasks::Request,
    "armonik.api.grpc.v1.agent.CreateTaskRequest.InitRequest"
        => armonik::agent::create_tasks::InitRequest,
    "armonik.api.grpc.v1.agent.CreateTaskReply" => armonik::agent::create_tasks::Response,
    "armonik.api.grpc.v1.agent.CreateTaskReply.CreationStatus"
        => armonik::agent::create_tasks::Status,
}

/// Project a message (recursively) onto the equivalence classes of its
/// armonik type, so that the semantic comparison reflects the documented
/// semantics. Applied to both sides of every round-trip.
pub fn normalize(message: &mut DynamicMessage) {
    let descriptor = message.descriptor();
    for field in descriptor.fields() {
        if !message.has_field(&field) {
            continue;
        }
        let mut value = message.get_field(&field).into_owned();
        if normalize_value(&mut value) {
            message.set_field(&field, value);
        }
    }
    apply_rules(message);
}

fn normalize_value(value: &mut Value) -> bool {
    match value {
        Value::Message(inner) => {
            normalize(inner);
            true
        }
        Value::List(items) => {
            let mut changed = false;
            for item in items {
                changed |= normalize_value(item);
            }
            changed
        }
        Value::Map(map) => {
            let mut changed = false;
            for item in map.values_mut() {
                changed |= normalize_value(item);
            }
            changed
        }
        _ => false,
    }
}

fn apply_rules(message: &mut DynamicMessage) {
    match message.descriptor().full_name() {
        // Absent or empty max_duration folds to the INFINITE_DURATION
        // default of `TaskOptions`.
        "armonik.api.grpc.v1.TaskOptions" => normalize_task_options(message),
        // Marker members only remember which member was set; oneofs whose
        // Rust `Default` is a member variant re-encode an absent oneof with
        // that member present — like the historical None => Default.
        "armonik.api.grpc.v1.DataChunk" => {
            normalize_bool_marker(message, "data_complete");
            normalize_default_member(message, "data");
        }
        "armonik.api.grpc.v1.InitKeyedDataStream" => {
            normalize_bool_marker(message, "last_result");
            normalize_default_member(message, "key");
        }
        "armonik.api.grpc.v1.InitTaskRequest" => {
            normalize_bool_marker(message, "last_task");
            normalize_default_member(message, "header");
        }
        "armonik.api.grpc.v1.Output" => normalize_default_member(message, "ok"),
        // Repeated pairs exposed as a map: order is lost and duplicate
        // statuses collapse (last wins).
        "armonik.api.grpc.v1.Count" => normalize_count(message),
        // The historical conversion drops the token when no member is set
        // (`Request::Invalid` has no slot for it).
        "armonik.api.grpc.v1.agent.CreateTaskRequest" => {
            if !any_member_set(message) {
                let token = field(message, "communication_token");
                message.clear_field(&token);
            }
        }
        // `Response`'s default is an empty `Error` carrying the token.
        "armonik.api.grpc.v1.agent.CreateTaskReply" => {
            normalize_default_member(message, "error");
        }
        "armonik.api.grpc.v1.agent.CreateTaskReply.CreationStatus" => {
            normalize_default_member(message, "error");
        }
        _ => {}
    }
}

fn field(message: &DynamicMessage, name: &str) -> prost_reflect::FieldDescriptor {
    message
        .descriptor()
        .get_field_by_name(name)
        .unwrap_or_else(|| panic!("field `{name}` exists"))
}

fn any_member_set(message: &DynamicMessage) -> bool {
    let descriptor = message.descriptor();
    let oneof = descriptor.oneofs().next().expect("flattened oneof exists");
    let member_set = oneof.fields().any(|member| message.has_field(&member));
    member_set
}

fn normalize_bool_marker(message: &mut DynamicMessage, member: &str) {
    let member = field(message, member);
    if message.has_field(&member) {
        message.set_field(&member, Value::Bool(true));
    }
}

fn normalize_default_member(message: &mut DynamicMessage, member: &str) {
    if !any_member_set(message) {
        let member = field(message, member);
        message.set_field(&member, Value::default_value_for_field(&member));
    }
}

fn normalize_task_options(message: &mut DynamicMessage) {
    let max_duration = field(message, "max_duration");
    let is_empty = match message.get_field(&max_duration).as_ref() {
        Value::Message(duration) => duration.encode_to_vec().is_empty(),
        _ => true,
    };
    if is_empty {
        let duration_desc = match max_duration.kind() {
            prost_reflect::Kind::Message(desc) => desc,
            other => panic!("max_duration should be a message, got {other:?}"),
        };
        let mut infinite = DynamicMessage::new(duration_desc);
        infinite.set_field_by_name("seconds", Value::I64(315576000000));
        message.set_field(&max_duration, Value::Message(infinite));
    }
}

/// Fold the repeated `StatusCount` pairs by status (last wins) and order
/// them, mirroring the `HashMap` representation.
fn normalize_count(message: &mut DynamicMessage) {
    let values = field(message, "values");
    if !message.has_field(&values) {
        return;
    }
    let Value::List(entries) = message.get_field(&values).into_owned() else {
        return;
    };
    let mut by_status = std::collections::BTreeMap::new();
    for entry in entries {
        let Value::Message(status_count) = &entry else {
            continue;
        };
        let status = field(status_count, "status");
        let key = match status_count.get_field(&status).as_ref() {
            Value::EnumNumber(number) => *number,
            _ => 0,
        };
        by_status.insert(key, entry);
    }
    message.set_field(&values, Value::List(by_status.into_values().collect()));
}
